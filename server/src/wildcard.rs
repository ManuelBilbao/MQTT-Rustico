pub fn mqtt_wildcard(topic: &str, wildcard: &str) -> bool {
    if topic == wildcard || wildcard == "#" {
        return true;
    }

    let topic_splited: Vec<&str> = topic.split("/").collect();
    let wildcard_splited: Vec<&str> = wildcard.split("/").collect();

    let mut i: usize = 0;
    for _ in 0..topic_splited.len() {
        if i < wildcard_splited.len() && wildcard_splited[i] == "+" {
            i += 1;
            continue;
        } else if i < wildcard_splited.len() && wildcard_splited[i] == "#" {
            return true;
        } else if i < wildcard_splited.len() && wildcard_splited[i] != topic_splited[i] {
            return false;
        }
        i += 1;
    }

    if i < wildcard_splited.len() && wildcard_splited[i] == "#" {
        i += 1;
    }

    return i == wildcard_splited.len();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test01_prueba_wildcard_no_debe_matchear() {
        let topic = String::from("sport/tennis/picnic/paco");
        let subscription = String::from("sport/+/+");
        assert!(!mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test02_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player1/ranking");
        let subscription = String::from("sport/tennis/player1/#");
        assert!(mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test03_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player1");
        let subscription = String::from("sport/tennis/player1/#");
        assert!(mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test04_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player1/score/wimbledon");
        let subscription = String::from("sport/tennis/player1/#");
        assert!(mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test05_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/pijama/coconut");
        let subscription = String::from("sport/tennis/+/coconut");
        assert!(mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test06_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player2/coconut");
        let subscription = String::from("sport/tennis/+/coconut");
        assert!(mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test07_prueba_wildcard_no_debe_matchear() {
        let topic = String::from("sport/tennis/player2/miranda/coconut");
        let subscription = String::from("sport/tennis/+/coconut");
        assert!(!mqtt_wildcard(&topic, &subscription));
    }

    #[test]
    fn test08_prueba_wildcard_no_debe_matchear() {
        let topic = String::from("sport/tennis");
        let subscription = String::from("sport/tennis/+");
        assert!(!mqtt_wildcard(&topic, &subscription));
    }
}
