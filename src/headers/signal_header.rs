use crate::EDFSpecifications;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SignalHeader {
    pub label: String,
    pub transducer: String,
    pub physical_dimension: String,
    pub physical_minimum: f64,
    pub physical_maximum: f64,
    pub digital_minimum: i32,
    pub digital_maximum: i32,
    pub prefilter: String,
    pub samples_count: usize,

    pub(crate) reserved: String,
}

impl SignalHeader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_annotation(size: usize, specification: EDFSpecifications) -> Self {
        let file_type = match specification {
            EDFSpecifications::EDF | EDFSpecifications::EDFPlus => "EDF",
            EDFSpecifications::BDF | EDFSpecifications::BDFPlus => "BDF",
        };

        let digital_range = match specification {
            EDFSpecifications::EDF | EDFSpecifications::EDFPlus => (-32768, 32767),
            EDFSpecifications::BDF | EDFSpecifications::BDFPlus => (-8388608, 8388607),
        };

        let sample_bytes = match specification {
            EDFSpecifications::EDF | EDFSpecifications::EDFPlus => 2,
            EDFSpecifications::BDF | EDFSpecifications::BDFPlus => 3,
        };

        Self {
            label: format!("{} Annotations", file_type),
            transducer: String::new(),
            physical_dimension: String::new(),
            digital_minimum: digital_range.0,
            digital_maximum: digital_range.1,
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            prefilter: String::new(),
            samples_count: size * sample_bytes, // TODO: Should those samples really be multiplied here?
            reserved: String::new(),
        }
    }

    pub fn with_label(&mut self, label: String) -> &mut Self {
        self.label = label;
        self
    }

    pub fn with_transducer(&mut self, transducer: String) -> &mut Self {
        self.transducer = transducer;
        self
    }

    pub fn with_physical_dimension(&mut self, physical_dimension: String) -> &mut Self {
        self.physical_dimension = physical_dimension;
        self
    }

    pub fn with_physical_range(&mut self, min: f64, max: f64) -> &mut Self {
        self.physical_minimum = min;
        self.physical_maximum = max;
        self
    }

    pub fn with_digital_range(&mut self, min: i32, max: i32) -> &mut Self {
        self.digital_minimum = min;
        self.digital_maximum = max;
        self
    }

    pub fn with_prefilter(&mut self, prefilter: String) -> &mut Self {
        self.prefilter = prefilter;
        self
    }

    pub fn with_samples_count(&mut self, samples_count: usize) -> &mut Self {
        self.samples_count = samples_count;
        self
    }

    pub fn is_annotation(&self) -> bool {
        self.label == "EDF Annotations" || self.label == "BDF Annotations"
    }

    pub fn annotation_char_bytes(&self) -> usize {
        if self.label == "EDF Annotations" {
            2
        } else if self.label == "BDF Annotations" {
            3
        } else {
            0
        }
    }

    pub fn to_digital_samples<S: Into<i32> + Copy>(&self, samples: &Vec<S>) -> Vec<i32> {
        samples.iter().map(|sample| {
            (*sample).into().clamp(self.digital_minimum, self.digital_maximum)
        }).collect()
    }

    pub fn to_physical_samples<S: Into<f64> + Copy>(&self, samples: &Vec<S>, range: f64, offset: f64) -> Vec<f64> {
        samples.iter().map(|sample| {
            let digital = (*sample).into();
            let physical = range * (offset + digital);
            physical.clamp(self.physical_minimum, self.physical_maximum)
        }).collect()
    }
}
