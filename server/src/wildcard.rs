pub fn compare_topic(topic_publish: &str, topic_subscribed: &str) -> bool {
    if *topic_publish == *topic_subscribed || topic_subscribed == "#" {
        return true;
    }

    let topic_splited: Vec<&str> = topic_publish.split('/').collect();
    let wildcard_splited: Vec<&str> = topic_subscribed.split('/').collect();

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

    i == wildcard_splited.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test01_prueba_wildcard_no_debe_matchear() {
        let topic = String::from("sport/tennis/picnic/paco");
        let subscription = String::from("sport/+/+");
        assert!(!compare_topic(&topic, &subscription));
    }

    #[test]
    fn test02_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player1/ranking");
        let subscription = String::from("sport/tennis/player1/#");
        assert!(compare_topic(&topic, &subscription));
    }

    #[test]
    fn test03_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player1");
        let subscription = String::from("sport/tennis/player1/#");
        assert!(compare_topic(&topic, &subscription));
    }

    #[test]
    fn test04_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player1/score/wimbledon");
        let subscription = String::from("sport/tennis/player1/#");
        assert!(compare_topic(&topic, &subscription));
    }

    #[test]
    fn test05_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/pijama/coconut");
        let subscription = String::from("sport/tennis/+/coconut");
        assert!(compare_topic(&topic, &subscription));
    }

    #[test]
    fn test06_prueba_wildcard_debe_matchear() {
        let topic = String::from("sport/tennis/player2/coconut");
        let subscription = String::from("sport/tennis/+/coconut");
        assert!(compare_topic(&topic, &subscription));
    }

    #[test]
    fn test07_prueba_wildcard_no_debe_matchear() {
        let topic = String::from("sport/tennis/player2/miranda/coconut");
        let subscription = String::from("sport/tennis/+/coconut");
        assert!(!compare_topic(&topic, &subscription));
    }

    #[test]
    fn test08_prueba_wildcard_no_debe_matchear() {
        let topic = String::from("sport/tennis");
        let subscription = String::from("sport/tennis/+");
        assert!(!compare_topic(&topic, &subscription));
    }
}
