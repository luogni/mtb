extern crate rusty_machine;
extern crate itertools;


use llpoint::LLPoint;
use self::rusty_machine::linalg::Matrix;
// use self::rusty_machine::learning::k_means::KMeansClassifier;
use self::rusty_machine::learning::UnSupModel;
use self::rusty_machine::learning::dbscan::DBSCAN;
use self::itertools::multizip;
use std::fs;
use std::path::Path;



fn get_data(all: &Vec<&LLPoint>) -> Matrix<f64> {
    let mut data = Vec::new();
    
    for a in all.into_iter() {
        data.extend_from_slice(&[a.lat() as f64, a.lon() as f64]);
    }

    Matrix::new(all.len(), 2, data)
}

// fn cluster_kmeans(all: &Vec<&LLPoint>) {
//     // Create a new model with 2 clusters
//     let mut model = KMeansClassifier::new(3);

//     let samples = get_data(&all);
//     // Train the model
//     println!("Training the model...");
//     // Our train function returns a Result<(), E>
//     model.train(&samples).unwrap();

//     let centroids = model.centroids().as_ref().unwrap();
//     println!("Model Centroids:\n{:.3}", centroids);

//     // Predict the classes and partition into
//     println!("Classifying the samples...");
//     let classes = model.predict(&samples).unwrap();
//     let (first, second): (Vec<usize>, Vec<usize>) = classes.data().iter().partition(|&x| *x == 0);

//     println!("Samples closest to first centroid: {}", first.len());
//     println!("Samples closest to second centroid: {}", second.len());
// }

fn cluster_dbscan(all: &Vec<&LLPoint>) -> Vec<Option<usize>> {
    let samples = get_data(&all);
    let mut model = DBSCAN::new(0.2, 1);
    model.train(&samples).unwrap();
    let clustering = model.clusters().unwrap();
    println!("Model Centroids:\n{:?}", clustering);
    let mut ret = Vec::new();
    for c in clustering {
        ret.push(c.clone())
    }
    ret
}

pub fn cluster(all: &Vec<LLPoint>) -> Vec<Option<usize>> {
    let points = all.into_iter().filter(|&p| p.is_latlon()).collect::<Vec<_>>();
    let names = all.into_iter().filter(|&p| p.is_latlon() == false).collect::<Vec<_>>();
    
    let ret = cluster_dbscan(&points);

    fs::remove_dir_all("./cluster").unwrap();
    
    for (p, &c) in multizip((&names, &ret)) {
        let d;
        match c {
            Some(cc) => {
                d = cc.to_string();
            },
            None => {
                d = String::from("none");
            }
        }
        let path = Path::new("./cluster").join(d);
        fs::create_dir_all(&path).unwrap();
        let path = path.join(Path::new(&p.name()).file_name().unwrap());
        fs::copy(p.name(), path).unwrap();
    }

    ret
}
