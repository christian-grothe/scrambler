#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum Subdivision {
    Quarter,          // Quarter note (1/4)
    Eighth,           // Eighth note (1/8)
    Sixteenth,        // Sixteenth note (1/16)
    TripletQuarter,   // Quarter-note triplet
    TripletEighth,    // Eighth-note triplet
    TripletSixteenth, // Sixteenth-note triplet
    DottedQuarter,    // Quarter note * 1.5
    DottedEighth,     // Eighth note * 1.5
    DottedSixteenth,  // Sixteenth note * 1.5
}

impl Subdivision {
    fn factor(self) -> f32 {
        match self {
            Subdivision::Quarter => 1.0,
            Subdivision::Eighth => 0.5,
            Subdivision::Sixteenth => 0.25,
            Subdivision::TripletQuarter => 2.0 / 3.0,
            Subdivision::TripletEighth => 1.0 / 3.0,
            Subdivision::TripletSixteenth => 1.0 / 6.0,
            Subdivision::DottedQuarter => 1.5,
            Subdivision::DottedEighth => 0.75,
            Subdivision::DottedSixteenth => 0.375,
        }
    }

    pub fn to_hz(self, bpm: f32) -> f32 {
        (bpm / 60.0) / self.factor()
    }

    pub fn get_symbol(&self) -> &str {
        match self {
            Subdivision::Quarter => "1/4",
            Subdivision::Eighth => "1/8",
            Subdivision::Sixteenth => "1/16",
            Subdivision::TripletQuarter => "1/4t",
            Subdivision::TripletEighth => "1/8t",
            Subdivision::TripletSixteenth => "1/16t",
            Subdivision::DottedQuarter => "1/4.",
            Subdivision::DottedEighth => "1/8.",
            Subdivision::DottedSixteenth => "1/16.",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Subdivision::Quarter => Subdivision::Eighth,
            Subdivision::Eighth => Subdivision::Sixteenth,
            Subdivision::Sixteenth => Subdivision::TripletQuarter,
            Subdivision::TripletQuarter => Subdivision::TripletEighth,
            Subdivision::TripletEighth => Subdivision::TripletSixteenth,
            Subdivision::TripletSixteenth => Subdivision::DottedQuarter,
            Subdivision::DottedQuarter => Subdivision::DottedEighth,
            Subdivision::DottedEighth => Subdivision::DottedSixteenth,
            Subdivision::DottedSixteenth => Subdivision::Quarter,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Subdivision::Quarter => Subdivision::DottedSixteenth,
            Subdivision::Eighth => Subdivision::Quarter,
            Subdivision::Sixteenth => Subdivision::Eighth,
            Subdivision::TripletQuarter => Subdivision::Sixteenth,
            Subdivision::TripletEighth => Subdivision::TripletQuarter,
            Subdivision::TripletSixteenth => Subdivision::TripletEighth,
            Subdivision::DottedQuarter => Subdivision::TripletSixteenth,
            Subdivision::DottedEighth => Subdivision::DottedQuarter,
            Subdivision::DottedSixteenth => Subdivision::DottedEighth,
        }
    }
}
