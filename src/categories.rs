pub fn new() -> Categories {
    Categories {
        subscriptions: Subscriptions {
            netflix: 0.0,
        },

        car: Car {
            payment: 0.0,
            insurance: 0.0,
        },

        groceries: Groceries {
            groceries: 0.0
        },

        house: House {
            payment: 0.0,
            insurance: 0.0,
        },

        saving: Saving {
            etf: 0.0,
        },

        uncategorized: Uncategorized {
            uncategorized: 0.0,
        },
    }
}

#[derive(Debug)]
struct Categories {
    pub subscriptions: Subscriptions,
    pub car: Car,
    pub house: House,
    pub groceries: Groceries,
    pub saving: Saving,
    pub uncategorized: Uncategorized,
}

#[derive(Debug)]
struct Subscriptions {
    pub netflix: f32,
}

#[derive(Debug)]
pub struct Car {
    pub payment: f32,
    pub insurance: f32,
}

#[derive(Debug)]
pub struct House {
    pub payment: f32,
    pub insurance: f32,
}

#[derive(Debug)]
pub struct Groceries {
    pub groceries: f32,
}

#[derive(Debug)]
pub struct Saving {
    pub etf: f32,
}

#[derive(Debug)]
pub struct Uncategorized {
    pub uncategorized: f32,
}
