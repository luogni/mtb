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
use errors::*;



fn get_data(all: &[&LLPoint]) -> Matrix<f64> {
    let mut data = Vec::new();
    
    for a in all {
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

fn cluster_dbscan(all: &[&LLPoint]) -> Result<Vec<Option<usize>>> {
    let samples = get_data(all);
    let mut model = DBSCAN::new(0.2, 1);
    try!(model.train(&samples).chain_err(|| "Error training dbscan"));
    let clustering = try!(model.clusters().ok_or("No cluster"));
    println!("Model Centroids:\n{:?}", clustering);
    let mut ret = Vec::<Option<usize>>::new();
    for c in clustering {
        ret.push(*c)
    }
    Ok(ret)
}

pub fn cluster(all: &[LLPoint]) -> Result<Vec<Option<usize>>> {
    let points = all.into_iter().filter(|&p| p.is_latlon()).collect::<Vec<_>>();
    let names = all.into_iter().filter(|&p| !p.is_latlon()).collect::<Vec<_>>();
    
    let ret = try!(cluster_dbscan(&points));

    try!(fs::remove_dir_all("./cluster").chain_err(|| "Can't work on cluster dir"));
    
    for (p, &c) in multizip((&names, &ret)) {
        let d = c.map_or(String::from("none"), |cc| cc.to_string());
        let path = Path::new("./cluster").join(d);

        try!(fs::create_dir_all(&path).chain_err(|| "Can't create dir"));
        Path::new(&p.name()).file_name().
            and_then(|p2| Some(path.join(p2))).
            and_then(|path| fs::copy(p.name(), path).ok());
    }
        
    Ok(ret)
}
