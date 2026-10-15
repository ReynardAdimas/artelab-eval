use crate::localizer::Localizer;
use barcode_scanner_rs::{
    detector::{BarcodeDetector, DecodedBarcode, RxingDetector},
    utils::cv_err,
};
use opencv::{
    core::{self, Mat, Point2f, Size, Vector},
    imgproc,
    prelude::*,
};

pub struct LocalizedRxingDetector {
    localizer: Box<dyn Localizer>,
    inner: RxingDetector,
    upscale: f64,
    margin_ratio: f64,
    local_otsu: bool,
    red_fallback: bool,
    debug_dir: Option<std::path::PathBuf>,
    debug_label: Option<String>,
    debug_counter: u64,
    pub crop_hits: u64,
    pub fallback_hits: u64,
    pub red_crop_hits: u64,
    pub red_full_hits: u64,
}

enum Src {
    Crop,
    Full,
}

impl LocalizedRxingDetector {
    pub fn new(
        localizer: Box<dyn Localizer>,
        upscale: f64,
        margin_ratio: f64,
        local_otsu: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            localizer,
            inner: RxingDetector::new(),
            upscale,
            margin_ratio,
            local_otsu,
            red_fallback: false,
            debug_dir: None,
            debug_label: None,
            debug_counter: 0,
            crop_hits: 0,
            fallback_hits: 0,
            red_crop_hits: 0,
            red_full_hits: 0,
        })
    }

    pub fn with_red_fallback(mut self, on: bool) -> Self {
        self.red_fallback = on;
        self
    }
    fn red_channel(frame: &Mat) -> Result<Option<Mat>, String> {
        if frame.channels() != 3 {
            return Ok(None);
        }
        let mut red = Mat::default();
        core::extract_channel(frame, &mut red, 2).map_err(cv_err)?;
        Ok(Some(red))
    }
    fn pass(
        &mut self,
        img: &Mat,
        quads: &[[Point2f; 4]],
        tag: &str,
    ) -> Result<(Vec<DecodedBarcode>, Src), String> {
        for q in quads {
            let crop = match self.crop_and_deskew(img, q) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let crop = self.local_otsu_gate(&crop)?;

            if let Some(dir) = &self.debug_dir {
                self.debug_counter += 1;
                let base = self
                    .debug_label
                    .clone()
                    .unwrap_or_else(|| "sample".to_string());
                let path = dir.join(format!("{base}_{tag}cand{:02}.png", self.debug_counter));
                opencv::imgcodecs::imwrite(path.to_str().unwrap(), &crop, &Vector::new())
                    .map_err(cv_err)?;
            }

            let out = self.inner.detect(&crop)?;
            if !out.is_empty() {
                return Ok((out, Src::Crop));
            }
        }
        Ok((self.inner.detect(img)?, Src::Full))
    }

    pub fn crop_and_deskew(&self, img: &Mat, quad: &[Point2f; 4]) -> Result<Mat, String> {
        let w = (((quad[2].x - quad[1].x).powi(2) + (quad[2].y - quad[1].y).powi(2)) as f64).sqrt();
        let h = (((quad[1].x - quad[0].x).powi(2) + (quad[1].y - quad[0].y).powi(2)) as f64).sqrt();
        if w < 1.0 || h < 1.0 {
            return Err("Quad candidate too small".into());
        }

        let mw = w * self.margin_ratio;
        let mh = h * self.margin_ratio;
        let s = self.upscale;
        let dst_w = ((w + 2.0 * mw) * s).round().max(1.0) as i32;
        let dst_h = ((h + 2.0 * mh) * s).round().max(1.0) as i32;

        let src: Vector<Point2f> = Vector::from_slice(quad);

        let dst: Vector<Point2f> = Vector::from_slice(&[
            Point2f::new((mw * s) as f32, ((h + mh) * s) as f32),
            Point2f::new((mw * s) as f32, (mh * s) as f32),
            Point2f::new(((w + mw) * s) as f32, (mh * s) as f32),
            Point2f::new(((w + mw) * s) as f32, ((h + mh) * s) as f32),
        ]);

        let m = imgproc::get_perspective_transform(&src, &dst, core::DECOMP_LU).map_err(cv_err)?;
        let mut out = Mat::default();
        imgproc::warp_perspective(
            img,
            &mut out,
            &m,
            Size::new(dst_w, dst_h),
            imgproc::INTER_LINEAR,
            core::BORDER_CONSTANT,
            core::Scalar::all(255.0),
        )
        .map_err(cv_err)?;

        Ok(out)
    }

    fn local_otsu_gate(&self, crop: &Mat) -> Result<Mat, String> {
        if !self.local_otsu {
            return crop.try_clone().map_err(cv_err);
        }

        let gray = if crop.channels() == 1 {
            crop.try_clone().map_err(cv_err)?
        } else {
            let mut g = Mat::default();
            imgproc::cvt_color_def(crop, &mut g, imgproc::COLOR_BGR2GRAY).map_err(cv_err)?;
            g
        };

        let mut bin = Mat::default();
        imgproc::threshold(
            &gray,
            &mut bin,
            0.0,
            255.0,
            imgproc::THRESH_BINARY | imgproc::THRESH_OTSU,
        )
        .map_err(cv_err)?;
        Ok(bin)
    }

    pub fn with_debug_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).ok();
        self.debug_dir = Some(dir);
        self
    }

    pub fn set_debug_label(&mut self, label: impl Into<String>) {
        self.debug_label = Some(label.into());
        self.debug_counter = 0;
    }
}

impl BarcodeDetector for LocalizedRxingDetector {
    fn detect(&mut self, frame: &Mat) -> Result<Vec<DecodedBarcode>, String> {
        let quads = self.localizer.locate(frame)?;

        let (out, src) = self.pass(frame, &quads, "")?;
        match src {
            Src::Crop => self.crop_hits += 1,
            Src::Full => self.fallback_hits += 1,
        }
        if !out.is_empty() || !self.red_fallback {
            return Ok(out);
        }

        if let Some(red) = Self::red_channel(frame)? {
            let (out, src) = self.pass(&red, &quads, "red_")?;
            if !out.is_empty() {
                match src {
                    Src::Crop => self.red_crop_hits += 1,
                    Src::Full => self.red_full_hits += 1,
                }
                return Ok(out);
            }
        }
        Ok(vec![])
    }
}