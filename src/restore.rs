use barcode_scanner_rs::utils::cv_err;
use ndarray::Array4;
use opencv::{
    core::{self, Mat, Size},
    imgproc,
    prelude::*,
};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};

const H: usize = 128;
const W: usize = 512;

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

pub struct Restorer {
    session: Session,
}

impl Restorer {
    pub fn new(model_path: &str) -> Result<Self, String> {
        let environment = ort::init().build().map_err(e)?;
        let session = Session::builder(&environment)
            .map_err(e)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(e)?
            .with_intra_threads(4)
            .map_err(e)?
            .commit_from_file(model_path)
            .map_err(e)?;
        Ok(Self { session })
    }

    pub fn restore(&mut self, crop: &Mat) -> Result<Mat, String> {
        let mut rgb = Mat::default();
        if crop.channels() == 1 {
            imgproc::cvt_color_def(crop, &mut rgb, imgproc::COLOR_GRAY2RGB).map_err(cv_err)?;
        } else {
            imgproc::cvt_color_def(crop, &mut rgb, imgproc::COLOR_BGR2RGB).map_err(cv_err)?;
        }
        if rgb.rows() > rgb.cols() {
            let mut rot = Mat::default();
            core::rotate(&rgb, &mut rot, core::ROTATE_90_CLOCKWISE).map_err(cv_err)?;
            rgb = rot;
        }
        let mut small = Mat::default();
        imgproc::resize(&rgb, &mut small, Size::new(W as i32, H as i32), 0.0, 0.0, imgproc::INTER_CUBIC)
            .map_err(cv_err)?;
        let bytes = small.data_bytes().map_err(cv_err)?;
        let input = Array4::<f32>::from_shape_fn((1, H, W, 3), |(_, y, x, c)| {
            bytes[(y * W + x) * 3 + c] as f32 / 127.5 - 1.0
        });

        let tensor = TensorRef::from_array_view(&input).map_err(e)?;
        let outputs = self.session.run(ort::inputs![tensor]).map_err(e)?;
        let y = outputs[0].try_extract_array::<f32>().map_err(e)?;

        let mut luma = vec![0u8; W * H];
        for yy in 0..H {
            for xx in 0..W {
                let r = (y[[0, yy, xx, 0]] + 1.0) * 127.5;
                let g = (y[[0, yy, xx, 1]] + 1.0) * 127.5;
                let b = (y[[0, yy, xx, 2]] + 1.0) * 127.5;
                luma[yy * W + xx] = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
            }
        }
        let m = Mat::from_slice(&luma).map_err(cv_err)?;
        let m = m.reshape(1, H as i32).map_err(cv_err)?;
        m.try_clone().map_err(cv_err)
    }
}