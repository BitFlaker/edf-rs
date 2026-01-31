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

    pub fn new_annotation(size: usize) -> Self {
        Self {
            label: "EDF Annotations".to_string(),
            transducer: String::new(),
            physical_dimension: String::new(),
            digital_minimum: -32768,
            digital_maximum: 32767,
            physical_minimum: -1.0,
            physical_maximum: 1.0,
            prefilter: String::new(),
            samples_count: size * 2,
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
        self.label == "EDF Annotations"
    }
}
