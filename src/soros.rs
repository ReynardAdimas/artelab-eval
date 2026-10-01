use crate::localizer::Localizer;
use barcode_scanner_rs::utils::cv_err;
use opencv::{
    core::{self, Mat, Point, Point2f, Scalar, Size, Vector, CV_32F, CV_32S, CV_8U},
    imgproc,
    prelude::*,
};

#[derive(Clone, Debug)]
pub struct SorosConfig {
    pub max_side: i32,
    pub window: i32,
    pub box_size: i32,
    pub sat_max: f32,      // 0..1; set 1.0 untuk menonaktifkan filter HSV (ablasi)
    pub eps: f32,
    pub thr_ratio: f32,
    pub top_k: usize,
    pub rel_peak: f32,
    pub min_peak: f32,
    pub min_area_ratio: f32,
}

impl Default for SorosConfig {
    fn default() -> Self {
        Self {
            max_side: 960, window: 7, box_size: 30, sat_max: 0.25, eps: 1e-6,
            thr_ratio: 0.3, top_k: 3, rel_peak: 0.5, min_peak: 0.02, min_area_ratio: 0.002,
        }
    }
}

struct Maps {
    s1: Mat,  // saliency 1D, CV_32F
    cxx: Mat, // untuk orientasi
    cyy: Mat,
    cxy: Mat,
}

pub struct SorosLocalizer {
    cfg: SorosConfig,
}

impl SorosLocalizer {
    pub fn new(cfg: SorosConfig) -> Self { Self { cfg } }

    fn blur(src: &Mat, k: i32) -> Result<Mat, String> {
        let mut dst = Mat::default();
        imgproc::blur(src, &mut dst, Size::new(k, k), Point::new(-1, -1), core::BORDER_DEFAULT)
            .map_err(cv_err)?;
        Ok(dst)
    }

    fn downscale(&self, frame: &Mat) -> Result<(Mat, f64), String> {
        let longest = frame.cols().max(frame.rows()) as f64;
        let scale = (self.cfg.max_side as f64 / longest).min(1.0);
        if scale >= 1.0 {
            return Ok((frame.try_clone().map_err(cv_err)?, 1.0));
        }
        let mut out = Mat::default();
        imgproc::resize(frame, &mut out, Size::default(), scale, scale, imgproc::INTER_AREA)
            .map_err(cv_err)?;
        Ok((out, scale))
    }

    fn maps(&self, small: &Mat) -> Result<Maps, String> {
        let c = &self.cfg;

        // 1. gray (u8) dan saturasi (u8, 0..255)
        let mut gray8 = Mat::default();
        let mut sat8: Option<Mat> = None;
        if small.channels() == 3 {
            imgproc::cvt_color_def(small, &mut gray8, imgproc::COLOR_BGR2GRAY).map_err(cv_err)?;
            let mut hsv = Mat::default();
            imgproc::cvt_color_def(small, &mut hsv, imgproc::COLOR_BGR2HSV).map_err(cv_err)?;
            let mut s = Mat::default();
            core::extract_channel(&hsv, &mut s, 1).map_err(cv_err)?;
            sat8 = Some(s);
        } else {
            // Input sudah grayscale -> informasi saturasi HILANG (lihat bagian 8)
            gray8 = small.try_clone().map_err(cv_err)?;
        }
        let mut gray = Mat::default();
        gray8.convert_to(&mut gray, CV_32F, 1.0 / 255.0, 0.0).map_err(cv_err)?;

        // 2. gradien
        let (mut ix, mut iy) = (Mat::default(), Mat::default());
        imgproc::sobel(&gray, &mut ix, CV_32F, 1, 0, 3, 1.0, 0.0, core::BORDER_DEFAULT).map_err(cv_err)?;
        imgproc::sobel(&gray, &mut iy, CV_32F, 0, 1, 3, 1.0, 0.0, core::BORDER_DEFAULT).map_err(cv_err)?;

        // 3. structure matrix (rata-rata di window)
        let prod = |a: &Mat, b: &Mat| -> Result<Mat, String> {
            let mut p = Mat::default();
            core::multiply(a, b, &mut p, 1.0, -1).map_err(cv_err)?;
            Self::blur(&p, c.window)
        };
        let cxx = prod(&ix, &ix)?;
        let cyy = prod(&iy, &iy)?;
        let cxy = prod(&ix, &iy)?;

        // 4-5. m1 (edge), m2 (corner), masker saturasi
        let (rows, cols) = (cxx.rows(), cxx.cols());
        let mut m1 = Mat::new_rows_cols_with_default(rows, cols, CV_32F, Scalar::all(0.0)).map_err(cv_err)?;
        let mut m2 = Mat::new_rows_cols_with_default(rows, cols, CV_32F, Scalar::all(0.0)).map_err(cv_err)?;
        {
            let (a, b, d) = (
                cxx.data_typed::<f32>().map_err(cv_err)?,
                cyy.data_typed::<f32>().map_err(cv_err)?,
                cxy.data_typed::<f32>().map_err(cv_err)?,
            );
            let sat = match &sat8 {
                Some(s) => Some(s.data_bytes().map_err(cv_err)?),
                None => None,
            };
            let sat_thr = (c.sat_max * 255.0).round() as u8;
            let o1 = m1.data_typed_mut::<f32>().map_err(cv_err)?;
            let o2 = m2.data_typed_mut::<f32>().map_err(cv_err)?;
            for i in 0..a.len() {
                if let Some(s) = sat {
                    if s[i] > sat_thr { continue; } // m1 = m2 = 0
                }
                let tr = a[i] + b[i];
                let den = tr * tr + c.eps;
                let diff = a[i] - b[i];
                o1[i] = (diff * diff + 4.0 * d[i] * d[i]) / den;
                o2[i] = 4.0 * (a[i] * b[i] - d[i] * d[i]) / den;
            }
        }

        // 6-7. box filter lalu s1 = max(0, B(m1) - B(m2))
        let (b1, b2) = (Self::blur(&m1, c.box_size)?, Self::blur(&m2, c.box_size)?);
        let mut s1 = Mat::new_rows_cols_with_default(rows, cols, CV_32F, Scalar::all(0.0)).map_err(cv_err)?;
        {
            let (x, y) = (b1.data_typed::<f32>().map_err(cv_err)?, b2.data_typed::<f32>().map_err(cv_err)?);
            let o = s1.data_typed_mut::<f32>().map_err(cv_err)?;
            for i in 0..x.len() { o[i] = (x[i] - y[i]).max(0.0); }
        }
        Ok(Maps { s1, cxx, cyy, cxy })
    }

    /// Susun ulang titik agar sisi quad[1]->quad[2] (lebar di crop_and_deskew)
    /// sejajar arah gradien dominan (melintang batang).
    fn order_for_scan(p: [Point2f; 4], theta: f64) -> [Point2f; 4] {
        let (dx, dy) = (theta.cos() as f32, theta.sin() as f32);
        let along = |a: Point2f, b: Point2f| {
            let (ex, ey) = (b.x - a.x, b.y - a.y);
            ((ex * dx + ey * dy) / (ex.hypot(ey) + 1e-6)).abs()
        };
        if along(p[1], p[2]) >= along(p[0], p[1]) { p } else { [p[1], p[2], p[3], p[0]] }
    }
}

impl Localizer for SorosLocalizer {
    fn locate(&mut self, frame: &Mat) -> Result<Vec<[Point2f; 4]>, String> {
        let (small, scale) = self.downscale(frame)?;
        let maps = self.maps(&small)?;
        let mut s1 = maps.s1.try_clone().map_err(cv_err)?;
        let min_area = self.cfg.min_area_ratio as f64 * (small.rows() * small.cols()) as f64;

        let mut out = Vec::new();
        let mut global_max = 0.0f64;

        let dbg_dir = std::env::var("SOROS_DEBUG_DIR").ok(); 
        let mut vis = Mat::default(); 
        if dbg_dir.is_some() {
            if small.channels() == 1 {
                imgproc::cvt_color_def(&small, &mut vis, imgproc::COLOR_GRAY2BGR).map_err(cv_err)?;
            } else {
                vis = small.try_clone().map_err(cv_err)?;
            }
        }

        for k in 0..self.cfg.top_k {
            let (mut mx, mut loc) = (0.0f64, Point::default());

            core::min_max_loc(&s1, None, Some(&mut mx), None, Some(&mut loc), &core::no_array())
                .map_err(cv_err)?;
            if std::env::var("SOROS_DEBUG").is_ok() {
                eprintln!("[soros] k={k} mx={mx:.4} loc=({}, {}) small=({}x{})", loc.x, loc.y, small.cols(), small.rows());
            }
            if k == 0 {
                global_max = mx;
                if mx < self.cfg.min_peak as f64 { break; } // tidak ada kode -> fallback
            } else if mx < self.cfg.rel_peak as f64 * global_max {
                break;
            }

            // 8. threshold relatif terhadap puncak -> komponen terhubung di puncak
            let mut bin32 = Mat::default();
            imgproc::threshold(&s1, &mut bin32, self.cfg.thr_ratio as f64 * mx, 255.0, imgproc::THRESH_BINARY)
                .map_err(cv_err)?;
            let mut bin8 = Mat::default();
            bin32.convert_to(&mut bin8, CV_8U, 1.0, 0.0).map_err(cv_err)?;
            let mut labels = Mat::default();
            imgproc::connected_components(&bin8, &mut labels, 8, CV_32S).map_err(cv_err)?;
            let l = *labels.at_2d::<i32>(loc.y, loc.x).map_err(cv_err)? as f64;
            let mut comp = Mat::default();
            core::in_range(&labels, &Scalar::all(l), &Scalar::all(l), &mut comp).map_err(cv_err)?;

            // hapus komponen dari s1 agar iterasi berikutnya mencari kandidat lain
            s1.set_to(&Scalar::all(0.0), &comp).map_err(cv_err)?; 

            if std::env::var("SOROS_DEBUG").is_ok() {
                eprintln!("[soros] comp_area={} min_area={min_area:.0}",
                core::count_non_zero(&comp).unwrap_or(-1));
            }

            if (core::count_non_zero(&comp).map_err(cv_err)? as f64) < min_area { continue; }

            // 9. kotak berputar minimal pada kontur terbesar
            let mut contours: Vector<Vector<Point>> = Vector::new();
            imgproc::find_contours(&comp, &mut contours, imgproc::RETR_EXTERNAL,
                                   imgproc::CHAIN_APPROX_SIMPLE, Point::new(0, 0)).map_err(cv_err)?;
            let Some(best) = contours.iter().max_by(|a, b| {
                let (aa, ab) = (
                    imgproc::contour_area(a, false).unwrap_or(0.0),
                    imgproc::contour_area(b, false).unwrap_or(0.0),
                );
                aa.partial_cmp(&ab).unwrap()
            }) else { continue };

            let rect = imgproc::min_area_rect(&best).map_err(cv_err)?;
            let mut bp = Mat::default();
            imgproc::box_points(rect, &mut bp).map_err(cv_err)?;
            let v = bp.data_typed::<f32>().map_err(cv_err)?;
            if v.len() != 8 { continue; }
            let p = [Point2f::new(v[0], v[1]), Point2f::new(v[2], v[3]), Point2f::new(v[4], v[5]), Point2f::new(v[6], v[7])];
            if dbg_dir.is_some() {
                let color = match k {
                    0 => Scalar::new(0.0, 0.0, 255.0, 0.0), // k0
                    1 => Scalar::new(0.0, 255.0, 0.0, 0.0), // k1
                    _ => Scalar::new(255.0, 0.0, 0.0,0.0) // k2
                };
                for i in 0..4 {
                    let a = Point::new(p[i].x as i32, p[i].y as i32); 
                    let b = Point::new(p[(i+1) % 4].x as i32, p[(i+1) % 4].y as i32);
                    imgproc::line(&mut vis, a, b, Scalar::new(0.0,0.0, 255.0, 0.0), 2, imgproc::LINE_8, 0).map_err(cv_err)?;
                }
                imgproc::put_text(&mut vis, &format!("k{k} {mx:.2}"), Point::new(p[0].x as i32, p[0].y as i32), imgproc::FONT_HERSHEY_SIMPLEX, 0.6, color, 2, imgproc::LINE_8, false).map_err(cv_err)?;
            }

            // orientasi dari structure matrix (rata-rata di komponen)
            let cxx = core::mean(&maps.cxx, &comp).map_err(cv_err)?[0];
            let cyy = core::mean(&maps.cyy, &comp).map_err(cv_err)?[0];
            let cxy = core::mean(&maps.cxy, &comp).map_err(cv_err)?[0];
            let theta = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
            let q = Self::order_for_scan(p, theta);

            // kembalikan ke koordinat frame asli
            let inv = (1.0 / scale) as f32;
            out.push([
                Point2f::new(q[0].x * inv, q[0].y * inv),
                Point2f::new(q[1].x * inv, q[1].y * inv),
                Point2f::new(q[2].x * inv, q[2].y * inv),
                Point2f::new(q[3].x * inv, q[3].y * inv),
            ]);
        }

        if let Some(dir) = dbg_dir {
            use std::sync::atomic::{AtomicUsize, Ordering}; 
            static N: AtomicUsize = AtomicUsize::new(0); 
            let n = N.fetch_add(1, Ordering::Relaxed);
            if n < 15 {
                std::fs::create_dir_all(&dir).ok(); 
                opencv::imgcodecs::imwrite(&format!("{dir}/soros_{n:03}.png"), &vis, &core::Vector::new()).map_err(cv_err)?;
            }
        } 
        Ok(out)
    }
}