use std::collections::HashMap;
use tracing::{error, info};

const DEFAULT_PORT: u16 = 7666;
const DEFAULT_DUMPFILE: &str = "dump.txt";
const DEFAULT_LOGFILE: &str = "logfile.txt";
const DEFAULT_IP: &str = "192.168.0.8";

pub struct Configuration {
    port: u16,
    dumpfile: String,
    logfile: String,
    ip: String,
}

impl Configuration {
    pub fn new() -> Self {
        Configuration {
            port: DEFAULT_PORT,
            dumpfile: DEFAULT_DUMPFILE.to_string(),
            logfile: DEFAULT_LOGFILE.to_string(),
            ip: DEFAULT_IP.to_string(),
        }
    }

    pub fn set_config(&mut self, file_path: &str) -> Result<(), String> {
        let map = self.parse(file_path)?;
        self.set_all_params(map)?;
        Ok(())
    }

    fn parse(&mut self, file_path: &str) -> Result<HashMap<String, String>, String> {
        let file: String = match std::fs::read_to_string(file_path) {
            Ok(file) => file,
            Err(_) => return Err("Error when trying to open the file".to_string()),
        };
        let mut map: HashMap<String, String> = HashMap::new();
        let lines = file.lines();

        for line in lines {
            let name_and_value: Vec<&str> = line.split('=').collect();
            let config_name: String = name_and_value[0]
                .to_lowercase()
                .replace(' ', "")
                .to_string();
            let value: String = name_and_value[1].replace(' ', "").to_string();
            map.insert(config_name, value);
        }
        Ok(map)
    }

    fn set_all_params(&mut self, map: HashMap<String, String>) -> Result<(), String> {
        if let Some(logfile_) = map.get("logfile") {
            self.logfile = logfile_.to_string();
            info!("Loaded log file : {}", self.logfile);
        }
        if let Some(port_) = map.get("port") {
            self.port = port_.parse().unwrap();
            match port_.parse() {
                Ok(port) => {
                    self.port = port;
                }
                Err(_) => {
                    error!("Error while parsing port from config file");
                    return Err("Error while parsing port from config file".into());
                }
            }
        }
        if let Some(dumpfile_) = map.get("dumpfile") {
            self.dumpfile = dumpfile_.to_string();
            info!("Uploaded archive configuration : {}", self.dumpfile);
        }
        Ok(())
    }

    pub fn get_address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    pub fn get_log_file(&self) -> String {
        self.logfile.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test01_direccion_correcta() {
        let mut aux = Configuration::new();
        aux.set_config("src/testcfg.txt").unwrap();
        assert_eq!(aux.get_address(), "127.0.0.1:1883");
    }

    #[test]
    fn test02_logfile_correcto() {
        let mut aux = Configuration::new();
        aux.set_config("src/testcfg.txt").unwrap();
        assert_eq!(aux.get_log_file(), "file.log");
    }
}
