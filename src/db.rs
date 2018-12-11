extern crate image;

use image::{imageops, ConvertBuffer};
use std::collections::BinaryHeap;
use std::cmp::{self, Eq, Ord, Ordering};
use std::fs;

const NUM_PIXELS: usize = 128;
const NUM_PIXELS2: usize = NUM_PIXELS * NUM_PIXELS;
const NUM_COEFFS: usize = 40;
const RESULTS: usize = 5;
const CHANNELS: usize = 3;

const BIN_Y: [f32; 6] = [5.04, 0.83, 1.01, 0.52, 0.47, 0.30];
const BIN_I: [f32; 6] = [19.21, 1.26, 0.44, 0.53, 0.28, 0.14];
const BIN_Q: [f32; 6] = [34.37, 0.36, 0.45, 0.14, 0.18, 0.27];

const WEIGHTS: [[f32; 6]; CHANNELS] = [BIN_Y, BIN_I, BIN_Q];

struct Image {
    y: Vec<f32>,
    i: Vec<f32>,
    q: Vec<f32>,
}

struct Signature {
    coeffs: [[SigVal; NUM_COEFFS]; CHANNELS],
    avg: [f32; CHANNELS],
}

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone)]
struct SigVal {
    data: f32,
    idx: u32,
}

#[derive(Debug)]
pub struct Index {
    paths: Vec<String>,
    avg: Vec<[f32; CHANNELS]>,
    pos: [Vec<Vec<usize>>; CHANNELS],
    neg: [Vec<Vec<usize>>; CHANNELS],
}

impl Eq for SigVal {}

impl Ord for SigVal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.partial_cmp(&other.data).unwrap()
    }
}

fn bin(i: usize, j: usize) -> usize {
    cmp::min(cmp::max(i, j), 5)
}

impl Image {
    fn new() -> Image {
        Image {
            y: vec![],
            i: vec![],
            q: vec![],
        }
    }

    fn haar2d(data: &mut [f32]) {
        let mut t = [0.0f32; NUM_PIXELS >> 1];

        // Decompose rows
        for i in (0..(NUM_PIXELS2)).step_by(NUM_PIXELS) {
            let mut c = 1.0f32;
            let mut h = NUM_PIXELS;
            while h > 1 {
                let h1 = h >> 1;
                c *= 0.7071; // 1/SQRT2
                let mut j1 = i;
                let mut j2 = i;
                for k in 0..h1 {
                    let j21 = j2+1;
                    // Difference with normalization factor
                    t[k] = (data[j2] - data[j21]) * c;
                    // Effective Average
                    data[j1] = data[j2] + data[j21];

                    j1+=1;
                    j2+=2;
                }
                // Overwrite data with differences
                data[i+h1..i+2*h1].copy_from_slice(&t[..h1]);

                h = h1;
            }
            data[i] *= c;
        }

        // Decompose columns
        for i in 0..NUM_PIXELS {
            let mut c = 1.0f32;
            let mut h = NUM_PIXELS;
            while h > 1 {
                let h1 = h >> 1;
                c *= 0.7071; // 1/SQRT2
                let mut j1 = i;
                let mut j2 = i;
                for k in 0..h1 {
                    let j21 = j2+NUM_PIXELS;
                    // Difference with normalization factor
                    t[k] = (data[j2] - data[j21]) * c;
                    // Effective Average
                    data[j1] = data[j2] + data[j21];

                    j1+=NUM_PIXELS;
                    j2+=2*NUM_PIXELS;
                }
                // Overwrite data with differences
                j1 = i+h1*NUM_PIXELS;
                for k in 0..h1 {
                    data[j1] = t[k];
                    j1+=NUM_PIXELS;
                }

                h = h1;
            }
            data[i] *= c;
        }
    }


    fn process_channel(data: &mut [f32], out: &mut [SigVal]) -> f32 {
        assert_eq!(out.len(), NUM_COEFFS);
        Image::haar2d(data);
        let avg = data[0] / 256.0 * 128.0;
        let mut heap = BinaryHeap::new();
        for (i, d) in data.iter().enumerate().skip(1) {
            heap.push(SigVal { data: -d.abs(), idx: i as u32 });
            if heap.len() > NUM_COEFFS {
                heap.pop();
            }
        }
        for (i, val) in heap.iter().enumerate() {
            out[i] = SigVal {
                data: data[val.idx as usize],
                idx: val.idx,
            };
        }

        avg
    }

    fn get_sig(mut self) -> Box<Signature> {
        let mut sig = Box::new(Signature {
            coeffs: [[SigVal{ idx: 0, data: 0.0}; NUM_COEFFS]; 3],
            avg: [0.0f32; 3],
        });

        sig.avg[0] = Image::process_channel(&mut self.y[..], &mut sig.coeffs[0][..]);
        sig.avg[1] = Image::process_channel(&mut self.i[..], &mut sig.coeffs[1][..]);
        sig.avg[2] = Image::process_channel(&mut self.q[..], &mut sig.coeffs[2][..]);

        sig
    }
}

impl Index {
    pub fn new() -> Index {
        Index {
            paths: Vec::new(),
            avg: Vec::new(),
            pos: [vec![vec![]; NUM_PIXELS2], vec![vec![]; NUM_PIXELS2], vec![vec![]; NUM_PIXELS2]],
            neg: [vec![vec![]; NUM_PIXELS2], vec![vec![]; NUM_PIXELS2], vec![vec![]; NUM_PIXELS2]],
        }
    }

    fn get_sig(data: &[u8], blur: bool) -> Box<Signature> {
        // Vec<u8>, format c1,c2,c3
        let input = image::load_from_memory(data).unwrap().to_rgb();

        // Convert to grayscale
        // let input: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = imageops::grayscale(&input).convert();
        let input = if blur {
            // input
            imageops::blur(&input, 2.0)
        } else {
            input
        };
        let mut pixels = imageops::resize(&input, NUM_PIXELS as u32, NUM_PIXELS as u32, image::FilterType::Gaussian)
            .into_raw();
        let mut img = Image::new();
        for (_i, pixel) in pixels.chunks_mut(3).enumerate() {
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            img.y.push(0.299 * r + 0.587 * g + 0.114 * b);
            img.i.push(0.596 * r - 0.275 * g - 0.321 * b);
            img.q.push(0.212 * r - 0.523 * g + 0.311 * b);
        }
        assert_eq!(img.y.len(), NUM_PIXELS2 as usize);

        img.get_sig()
    }

    pub fn update(&mut self, path: &str) {
        let idx = self.paths.len();
        let buf = fs::read(path).unwrap();
        let sig = Index::get_sig(&buf, false);
        self.paths.push(path.to_owned());
        self.avg.push([sig.avg[0], sig.avg[1], sig.avg[2]]);
        for c in 0..CHANNELS {
            for val in sig.coeffs[c].iter() {
                if val.data > 0.0 {
                    self.pos[c][val.idx as usize].push(idx);
                } else {
                    self.neg[c][val.idx as usize].push(idx);
                }
            }
        }
    }

    pub fn query(&self, path: &str) -> Vec<(&str, f32)> {
        let buf = fs::read(path).unwrap();
        self.query_buf(&buf)
    }

    pub fn query_buf(&self, data: &[u8]) -> Vec<(&str, f32)> {
        let mut scores = vec![(0, 0.0f32); self.paths.len()];
        let sig = Index::get_sig(data, true);
        for (i, avg) in self.avg.iter().enumerate() {
            let mut s = 0.0;
            for c in 0..CHANNELS {
                s += WEIGHTS[c][0] * (sig.avg[c] - avg[c]).abs();
            }
            scores[i] = (i, s);
        }

        for idx in 0..NUM_COEFFS {
            for c in 0..CHANNELS {
                let mut l = Vec::new();
                let cidx = sig.coeffs[c][idx].idx as usize;
                let i = cidx / NUM_PIXELS;
                let j = cidx % NUM_PIXELS;
                if sig.coeffs[c][idx].data.abs() > 0.001f32 {
                    if sig.coeffs[c][idx].data > 0.0f32 {
                        l.push(((i, j), &self.pos[c][cidx]));
                    } else {
                        l.push(((i, j), &self.neg[c][cidx]));
                    }
                }
                for ((i, j), idxs) in l {
                    for idx in idxs {
                        scores[*idx].1 -= WEIGHTS[c][bin(i, j)];
                    }
                }
            }
        }

        scores.sort_by_key(|a| a.1 as isize);
        scores.into_iter()
            .take(RESULTS)
            .map(|(i, s)| (&self.paths[i][..], s))
            .collect()
    }
}
