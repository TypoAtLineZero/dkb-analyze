use std::collections::HashMap;

pub fn get_categories() -> HashMap<&'static str, Vec<&'static str>> {
    let mut categories = HashMap::new();
    categories.insert("Food", vec!["restaurant", "grocery", "cafe"]);
    categories.insert("Transportation", vec!["bus", "taxi", "fuel", "train"]);
    categories.insert("Entertainment", vec!["movie", "concert", "game"]);
    categories
}
