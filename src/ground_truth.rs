use barcode_scanner_rs::utils::cv_err; 
use opencv::{core::Mat, prelude::*, imgcodecs};
use std::{fs, path::{Path, PathBuf}}; 

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubDataset {
    AutoFocus,
    NoAutoFocus,
    Muenster, 
    DealKaist,
} 

impl SubDataset {
    pub fn label(self) -> &'static str {
        match self {
            SubDataset::AutoFocus => "autofocus",
            SubDataset::NoAutoFocus => "no_autofocus",
            SubDataset::Muenster => "muenster", 
            SubDataset::DealKaist => "DealKaist"
        }
    }
}

pub struct Sample {
    pub file: String, 
    pub sub : SubDataset, 
    pub expected: String, 
    // pub img: Mat, 
    pub path: PathBuf
} 

impl Sample {
    pub fn load_image(&self) -> Result<Mat, String> {
        let img = imgcodecs::imread(self.path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)
            .map_err(cv_err)?; 
        if img.empty() {
            return Err(format!("empty image: {}", self.file));
        }
        Ok(img)
    }
}

type LabelFn = fn(&Path) -> Option<String>;

fn label_from_filename(image_path: &Path) -> Option<String> {
    let stem = image_path.file_stem()?.to_str()?;
    let code: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
    if code.is_empty() { None } else { Some(code) }
}

fn label_from_txt(image_path: &Path) -> Option<String> {
    let mut txt_path = image_path.as_os_str().to_os_string();
    txt_path.push(".txt");
    let content = fs::read_to_string(Path::new(&txt_path)).ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}


fn load_folder(root: &Path, sub: SubDataset, label_fn: LabelFn) -> Result<Vec<Sample>, String> {
    let mut out = Vec::new();
    let mut skip_no_label = 0usize;

    let mut paths: Vec<_> = fs::read_dir(root)
        .map_err(|e| format!("{root:?}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    paths.sort(); 

    for path in paths {
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue };
        if !matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png") {
            continue;
        }

        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let Some(expected) = label_fn(&path) else {
            skip_no_label += 1;
            eprintln!("Skipping {name}: label tidak ditemukan");
            continue;
        };

        let img = imgcodecs::imread(path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)
            .map_err(cv_err)?;
        if img.empty() {
            eprintln!("Skipping {name} because the image is empty");
            continue;
        }
        out.push(Sample { file: name, sub, expected, path: path.clone() });
    }

    if out.is_empty() {
        return Err(format!("No valid samples found in {root:?}"));
    }
    if skip_no_label > 0 {
        eprintln!("Skipped {skip_no_label} files in {root:?} (no label)");
    }
    Ok(out)
}


pub fn load_all(data_root: &Path) -> Result<Vec<Sample>, String> {
    let mut all = load_folder(
        &data_root.join("dataset1_autofocus"),
        SubDataset::AutoFocus,
        label_from_txt,
    )?;
    all.extend(load_folder(
        &data_root.join("dataset2_no_autofocus"),
        SubDataset::NoAutoFocus,
        label_from_txt,
    )?);
    Ok(all)
} 

pub fn load_muenster(muenster_root: &Path) -> Result<Vec<Sample>, String> {
    let all = load_folder(muenster_root, SubDataset::Muenster, label_from_filename)?;
    Ok(all)
}

pub fn load_deal_kaist(root: &Path) -> Result<Vec<Sample>, String> {
    let all = load_folder(
        &root.join("single_test"),
        SubDataset::DealKaist,
        label_from_filename,
    )?;
    Ok(all)
}