References Benchmark : https://www.dynamsoft.com/codepool/barcode-scanning-accuracy-benchmark-and-comparison.html 


## Combination Tested
| Label | Preprocessing | Decoder |
|---|---|---|
| `gray+localize+rxing+red` | grayscale, decoder crops internally | localizing -> crop -> rxing |
| `gray+localize_otsu+rxing+red` | grayscale, decoder crops internally | localize -> crop -> Otsu on crop -> rxing |
| `raw+localize_otsu+rxing+red` | none, decoder crops internally | localizing -> crop -> rxing |

## Dataset : 	Artelab Medium Barcode 1D Collection 

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| autofocus | gray+localize+rxing+red | 215 | 96.74% | 97.29% | 0.0% | 1222.80 | 3491.43 |
| autofocus | gray+localize_otsu+rxing+red | 215 | 97.21% | 95.54% | 0.0% | 1151.55 | 3682.08 |
| autofocus | raw+localize+rxing+red | 215 | 98.14% | 97.31% | 0.0% | 1372.09 | 5045.94 |
| no_autofocus | gray+localize+rxing+red | 215 | 39.07% | 87.50% | 0.0% | 174.45 | 327.50 |
| no_autofocus | gray+localize_otsu+rxing+red | 215 | 30.70% | 82.50% | 0.0% | 153.78 | 363.13 |
| no_autofocus | raw+localize+rxing+red | 215 | 40.93% | 87.13% | 0.0% | 254.39 | 534.44 | 

### Comparison against the published benchmark (same dataset)
 
| SDK | Autofocus reading rate | Autofocus precision | No-autofocus reading rate | No-autofocus precision | 
|---|---:|---:|---:|---:|
| Dynamsoft Barcode Reader | 100% | 100% | 81.86% | 100% |
| Scandit | 91.63% | 100%% | 79.07% | 100%
| **ZXing-CPP** | **82.36%** | **99.44%** | **10.23%** | **91.67%** |
| pyZbar | 89.77% | 99.48% | 13.95% | 78.95% |
| **raw+localize+rxing** | **96.74%** | **96.00%** | **40.00%** | **86.87%** | 
| **raw+localize+rxing** | **98.14%** | **97.31%** | **40.93%** | **87.13%** | 


## Dataset : 	Muenster BarcodeDB  

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| muenster | gray+localize+rxing+red | 1055 | 88.44% | 96.50% | 0.0% | 109.04 | 386.82 |
| muenster | gray+localize_otsu+rxing+red | 1055 | 87.87% | 95.68% | 0.0% | 115.27 | 407.87 |
| muenster | raw+localize+rxing+red | 1055 | 89.00% | 95.75% | 0.0% | 137.31 | 469.12 |

### Comparison against the published benchmark (same dataset)
 
| SDK | Reading rate | Precision |
|---|---:|---:|
| Dynamsoft Barcode Reader | 96.96% | 100% |
| Scandit | 93.26% | 100% |
| **ZXing-CPP** | **75.14%** | **99.87%** |
| pyZbar | 70.59% | 95.63% |
| **gray+localize+rxing** | **88.25%** | **94.74%** |
| **gray+localize+rxing** | **89.00%** | **95.75%** | 

## Dataset : 	DEAL Lab Barcode Dataset 

**Result so far:**
| sub-dataset | combo | n | reading_rate | precision | err% | p50 (ms) | p95 (ms) |
|---|---|---:|---:|---:|---:|---:|---:|
| muenster | gray+localize+rxing+red | 2000 | 75.40% | 92.75% | 0.0% | 148.95 | 2509.30 |
| muenster | gray+localize_otsu+rxing+red | 2000 | 73.75% | 90.92% | 0.0% | 129.05 | 1856.68 |
| muenster | raw+localize+rxing+red | 2000 | 78.40% | 91.85% | 0.0% | 164.61 | 2139.18 |

### Comparison against the published benchmark (same dataset)
 
| SDK | Reading rate | Precision |
|---|---:|---:|
| Dynamsoft Barcode Reader | 91.95% | 100% |
| Scandit | 61.05% | 100% |
| **ZXing-CPP** | **63.20%** | **97.98%** |
| pyZbar | 72.60% | 99.38% |
| **gray+localize+rxing** | **75.55%** | **91.40%** |
| **raw+localize+rxing+red** | **78.40%** | **91.85%** | 