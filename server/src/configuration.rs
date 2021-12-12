use std::collections::HashMap;
use tracing::{error, info};

const DEFAULT_PORT: u16 = 7666;
const DEFAULT_LOGFILE: &str = "logfile.txt";
const DEFAULT_IP: &str = "127.0.0.1";
const DEFAULT_PASSWORD: bool = true;

pub struct Configuration {
    port: u16,
    logfile: String,
    ip: String,
    pub password: bool,
}

impl Configuration {
    pub fn new() -> Self {
        Configuration {
            port: DEFAULT_PORT,
            logfile: DEFAULT_LOGFILE.to_string(),
            ip: DEFAULT_IP.to_string(),
            password: DEFAULT_PASSWORD,
        }
    }

    pub fn set_config(&mut self, file_path: &str) -> Result<(), String> {
        let map = self.parse(file_path)?;
        self.set_all_params(map)?;
        Ok(())
    }

    fn check_number_between(&mut self, number: &str, bottom: u32, top: u32) -> bool {
        let int_number: u32;
        match number.parse::<u32>() {
            Ok(x) => int_number = x,
            Err(_) => return false,
        }

        if int_number <= top && int_number >= bottom {
            return true;
        }
        false
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
            if !self.check_number_between(port_, 0, 65536) {
                return Err("Unvalid port".to_string());
            }
            match port_.parse::<u16>() {
                Ok(port) => {
                    self.port = port;
                }
                Err(_) => {
                    error!("Error while parsing ip from config file");
                    return Err("Error while parsing ip from config file".into());
                }
            }
        }
        if let Some(ip_) = map.get("ip") {
            self.ip = ip_.to_string();
        }
        if let Some(password_) = map.get("password") {
            match password_.parse::<u32>() {
                Ok(pass) => {
                    if pass == 0 {
                        self.password = false
                    }
                }
                Err(_) => {
                    error!("Error while parsing password from config file");
                    return Err("Error while parsing password from config file".into());
                }
            }
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
