use barcode_scanner_rs::utils::cv_err; 
use opencv::{core::Mat, prelude::*, imgcodecs};
use std::{fs, path::Path}; 

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubDataset {
    AutoFocus,
    NoAutoFocus
} 

impl SubDataset {
    pub fn label(self) -> &'static str {
        match self {
            SubDataset::AutoFocus => "autofocus",
            SubDataset::NoAutoFocus => "no_autofocus",
        }
    }
}

pub struct Sample {
    pub file: String, 
    pub sub : SubDataset, 
    pub expected: String, 
    pub img: Mat
} 

fn read_txt(image_path: &Path) -> Option<String> {
    let mut txt_path = image_path.as_os_str().to_os_string();
    txt_path.push(".txt");
    let txt_path = Path::new(&txt_path); 
    let content = fs::read_to_string(txt_path).ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
} 

fn load_folder(root: &Path, sub: SubDataset) -> Result<Vec<Sample>, String> {
    let mut out = Vec::new(); 
    let mut skip_no_txt = 0usize;
    let entries = fs::read_dir(root).map_err(|e| format!("{root:?}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path(); 
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {continue};
        if !matches!(ext.to_lowercase().as_str(), "jpg") {
            continue;
        } 

        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let Some(expected) = read_txt(&path) else {
            skip_no_txt += 1; 
            eprintln!("Skipping {name} because no corresponding .txt file was found");
            continue;
        }; 

        let img = imgcodecs::imread(path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)
            .map_err(cv_err)?; 
        if img.empty() {
            eprintln!("Skipping {name} because the image is empty");
            continue;
        }
        out.push(Sample {file: name, sub, expected, img});

    }
    if out.is_empty() {
        return Err(format!("No valid samples found in {root:?}"));
    }
    if skip_no_txt > 0 {
        eprintln!("Skipped {skip_no_txt} files because no corresponding .txt file was found");
    }
    Ok(out)
} 

pub fn load_all(data_root: &Path) -> Result<Vec<Sample>, String> {
    let mut all = load_folder(&data_root.join("dataset1_autofocus"), SubDataset::AutoFocus)?;
    all.extend(load_folder(&data_root.join("dataset2_no_autofocus"), SubDataset::NoAutoFocus)?);
    println!("Loaded: {} autofocus, {} no autofocus", all.iter().filter(|s| s.sub == SubDataset::AutoFocus).count(), all.iter().filter(|s| s.sub == SubDataset::NoAutoFocus).count());
    Ok(all)
}