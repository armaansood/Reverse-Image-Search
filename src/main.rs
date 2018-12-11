extern crate image;
extern crate glob;
extern crate pbr;
extern crate tiny_http;
extern crate rand;

mod db;

use db::Index;
use std::io;
use rand::Rng;

const COUNT: u64 = 10000;

fn main() -> io::Result<()> {
    let mut index = Index::new();

    let mut pb = pbr::ProgressBar::new(COUNT);
    let mut imgs = vec![];
    for entry in glob::glob("data/caltech101/**/*.jpg").unwrap() {
        imgs.push(entry.unwrap());
    }

    imgs.sort();
    for img in imgs.iter().take(COUNT as usize) {
        index.update(img.as_path().to_str().unwrap());
        pb.inc();
    }

    index.update("data/mona.jpg");
    index.update("data/flower.jpg");
    index.update("data/flower2.jpg");
    index.update("data/face2.jpg");
    index.update("data/face3.jpg");

    println!("{:?}", index.query("data/mona-noise.jpg"));
    println!("{:?}", index.query("data/acc-noise.jpg"));
    rand::thread_rng().shuffle(&mut imgs);
    let mut scores = 0.0;
    for img in imgs.iter().take(100) {
        let path = img.as_path().to_str().unwrap();
        let res = index.query(path);
        let mut score = 1.0;
        for (p, _) in res {
            if p == path {
                break;
            }
            score /= 2.0;
        }
        if score != 1.0 {
            println!("incorrect: {}, {}", path, score);
        }
        scores += score;
    }
    println!("Avg Score: {}", scores/100.0);

    let server = tiny_http::Server::http("0.0.0.0:1080").unwrap();
    loop {
        let mut request = match server.recv() {
            Ok(rq) => rq,
            Err(e) => { println!("error: {}", e); break }
        };
        let mut buf = Vec::new();
        request.as_reader().read_to_end(&mut buf)?;
        let res = index.query_buf(&buf);
        // JSON :)
        let mut resp = "[".to_owned();
        for (path, score) in res {
            resp += &format!("[\"{}\", {}],", path, score);
        }
        // Remove the last comma
        resp.pop();
        resp += "]";
        let response = tiny_http::Response::from_string(resp);
        request.respond(response)?;
    }

    Ok(())
}
