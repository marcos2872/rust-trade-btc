#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CsvBtcFile {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: String,
}

pub struct ReaderBtcFile {}

impl ReaderBtcFile {
    pub fn read_btc_csv_file(
        file_path: &str,
    ) -> Result<Vec<CsvBtcFile>, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file_path)?;
        let reader = std::io::BufReader::new(file);
        let mut csv_reader = csv::Reader::from_reader(reader);

        let mut data = Vec::new();
        for result in csv_reader.deserialize() {
            let record: CsvBtcFile = result?;

            data.push(record);
        }

        Ok(data)
    }
}
