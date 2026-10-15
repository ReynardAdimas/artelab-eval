use barcode_scanner_rs::utils::cv_err;
use opencv::{
    core::{Mat, Point2f, Vector}, 
    objdetect::{BarcodeDetector as CvBarcodeDetector, GraphicalCodeDetectorTrait}, 
    prelude::*
}; 

pub trait Localizer {
    fn locate(&mut self, frame: &Mat) -> Result<Vec<[Point2f; 4]>, String>;
}

pub struct OpenCvLocalizer {
    inner : CvBarcodeDetector
} 

impl OpenCvLocalizer {
    pub fn new() -> Result<Self, String> {
        Ok(Self { inner: CvBarcodeDetector::default().map_err(cv_err)? })
    }
} 

impl Localizer for OpenCvLocalizer {
    fn locate(&mut self, frame: &Mat) -> Result<Vec<[Point2f; 4]>, String> {
        let mut points: Vector<Point2f> = Vector::new(); 
        let found = self.inner.detect(frame, &mut points).map_err(cv_err)?;
        if !found || points.is_empty() {
            return Ok(vec![]);
        } 
        let pts = points.to_vec(); 
         Ok(pts
            .chunks(4)
            .filter(|q| q.len() == 4)
            .map(|q| [q[0], q[1], q[2], q[3]])
            .collect()) 
        }
}