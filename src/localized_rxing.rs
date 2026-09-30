use barcode_scanner_rs:: {
    detector::{BarcodeDetector, DecodedBarcode, RxingDetector}, 
    utils::cv_err
};
use opencv::{
    core::{self, Mat, Point2f, Size, Vector}, 
    imgproc, 
    objdetect::{BarcodeDetector as CvBarcodeDetector, GraphicalCodeDetectorTrait, GraphicalCodeDetectorTraitConst}, 
    prelude::*
}; 

pub struct LocalizedRxingDetector {
    localizer: CvBarcodeDetector, 
    inner: RxingDetector, 
    upscale: f64, 
    margin_ratio: f64, 
    local_otsu: bool
} 

impl LocalizedRxingDetector {
    pub fn new(upscale: f64, margin_ratio: f64, local_otsu: bool) -> Result<Self, String> {
        Ok(Self {
            localizer: CvBarcodeDetector::default().map_err(cv_err)?, 
            inner: RxingDetector::new(), 
            upscale, 
            margin_ratio, 
            local_otsu
        })
    } 

    fn crop_and_deskew(&self, img: &Mat, quad: &[Point2f; 4]) -> Result<Mat, String> {
        let w = (((quad[2].x - quad[1].x).powi(2) + (quad[2].y - quad[1].y).powi(2)) as f64).sqrt();
        let h = (((quad[1].x - quad[0].x).powi(2) + (quad[1].y - quad[0].y).powi(2)) as f64).sqrt(); 
        if w < 1.0 || h < 1.0 {
            return Err("Quad candidate too small".into());
        } 

        let mw = w*self.margin_ratio; 
        let mh = h*self.margin_ratio; 
        let dst_w = ((w+2.0*mw) * self.upscale).round().max(1.0) as i32; 
        let dst_h = ((h+2.0*mw) * self.upscale).round().max(1.0) as i32; 
        let s = self.upscale; 

        let src: Vector<Point2f> = Vector::from_slice(quad); 

        let dst: Vector<Point2f> = Vector::from_slice(&[
            Point2f::new((mw*s) as f32, ((h+mh)*s) as f32), 
            Point2f::new((mw * s) as f32, (mh*s) as f32), 
            Point2f::new(((w+mw)*s) as f32, (mh*s) as f32), 
            Point2f::new(((w+mw)*s) as f32, ((h+mh)*s) as f32)
        ]); 

        let m = imgproc::get_perspective_transform(&src, &dst, core::DECOMP_LU).map_err(cv_err)?; 
        let mut out = Mat::default(); 
        imgproc::warp_perspective(img, &mut out, &m, Size::new(dst_w, dst_h), imgproc::INTER_LINEAR, core::BORDER_CONSTANT, core::Scalar::all(255.0),).map_err(cv_err)?;


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
        imgproc::threshold(&gray, &mut bin, 0.0, 255.0, imgproc::THRESH_BINARY | imgproc::THRESH_OTSU).map_err(cv_err)?;
        Ok(bin)
    }
} 

impl BarcodeDetector for LocalizedRxingDetector {
    fn detect(&mut self, frame: &Mat) -> Result<Vec<DecodedBarcode>, String> {
        let mut points: Vector<Point2f> = Vector::new(); 
        let found = self.localizer.detect(frame, &mut points).map_err(cv_err)?;
        if !found || points.is_empty() {
            return self.inner.detect(frame);
        } 

        let pts: Vec<Point2f> = points.to_vec();
        let mut out = Vec::new(); 
        for quad in pts.chunks(4) {
            if quad.len() != 4 {
                continue;
            } 
            let q: [Point2f; 4] = [quad[0], quad[1], quad[2], quad[3]]; 
            let crop = match self.crop_and_deskew(frame, &q) {
                Ok(c) => c, 
                Err(_) => continue,
            };
            let crop = self.local_otsu_gate(&crop)?;
            out.extend(self.inner.detect(&crop)?);
        }

        if out.is_empty() {
            return self.inner.detect(frame);
        }
        Ok(out)
    }
}