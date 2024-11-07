use polars_core::utils::rayon::option;
use std::fs;

pub fn get_keywords () {
    let raw_keywords = fs::read_to_string("./subscription").expect("Unable to read file");
    println!("{}", raw_keywords);

    let keywords: Vec<&str> = raw_keywords.split(' ').collect();

    for keyword in keywords {
        println!("{}", keyword);
    }
}


// struct subscriptions {
//         amount: 0.0,
//         keywords: keywords_subscription, 
//     }

    //     car: Car {
    //         amount: 0.0,
    //         keywords: keywords_car,
    //     },
        
    //     groceries: Groceries {
    //         amount: 0.0,
    //         keywords: keywords_groceries,
    //     },
        
    //     house: House {
    //         amount: 0.0,
    //         keywords: keywords_house,
    //     },
        
    //     saving: Saving {
    //         amount: 0.0,
    //         keywords: keywords_saving,
    //     },
        
    //     uncategorized: Uncategorized {
            // amount: 0.0,
            // missingKeywords: 
    //     },
    // }


// #[derive(Debug)]
// pub struct Categories {
//     pub subscriptions: Subscriptions,
//     pub car: Car,
//     pub house: House,
//     pub groceries: Groceries,
//     pub saving: Saving,
//     pub uncategorized: Uncategorized,
// }

// #[derive(Debug)]
// pub struct Subscriptions {
//     pub amount: f32,
//     keywords: [&str; 2]
// }

// #[derive(Debug)]
// pub struct Car {
//     pub amount: f32,
//     keywords: Option<String>,
// }

// #[derive(Debug)]
// pub struct House {
//     pub amount: f32,
//     keywords: Option<String>,
// }

// #[derive(Debug)]
// pub struct Groceries {
//     pub amount: f32,
//     keywords: Option<String>
// }

// #[derive(Debug)]
// pub struct Saving {
//     pub amount: f32,
//     keywords: Option<String>
// }

// #[derive(Debug)]
// pub struct Uncategorized {
//     pub amount: f32,
// }
