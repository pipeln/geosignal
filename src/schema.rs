use crate::types::Nap;
use serde_json::Value;

const LOCAL_TYPES: &[&str] = &[
    "localbusiness",
    "restaurant",
    "dentist",
    "plumber",
    "electrician",
    "locksmith",
    "hairsalon",
    "beautysalon",
    "dayspa",
    "healthclub",
    "store",
    "homeandconstructionbusiness",
    "professionalservice",
    "legalservice",
    "accountingservice",
    "autorepair",
    "autodealer",
    "bakery",
    "cafeorcoffeeshop",
    "barorpub",
    "hotel",
    "lodgingbusiness",
    "medicalbusiness",
    "medicalclinic",
    "physician",
    "veterinarycare",
    "realestateagent",
    "insuranceagency",
    "financialservice",
    "travelagency",
    "movingcompany",
    "roofingcontractor",
    "hvacbusiness",
    "generalcontractor",
    "housepainter",
    "pestcontrol",
    "cleaningservice",
    "jewelrystore",
    "furniturestore",
    "clothingstore",
    "hardwarestore",
    "florist",
    "gym",
    "childcare",
    "drycleaner",
    "optician",
    "pharmacy",
];

fn type_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(|text| text.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Walk plain objects, including @graph containers, to a bounded depth.
pub fn walk_nodes(values: &[Value], out: &mut Vec<Value>, depth: usize) {
    if depth > 5 {
        return;
    }
    for value in values {
        match value {
            Value::Array(items) => walk_nodes(items, out, depth + 1),
            Value::Object(map) => {
                out.push(value.clone());
                if let Some(graph) = map.get("@graph") {
                    if let Some(items) = graph.as_array() {
                        walk_nodes(items, out, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn collect_types(values: &[Value]) -> Vec<String> {
    let mut types: Vec<String> = Vec::new();
    let mut nodes = Vec::new();
    walk_nodes(values, &mut nodes, 0);
    for node in &nodes {
        for item in type_list(node.get("@type")) {
            if !types.iter().any(|existing| existing == &item) {
                types.push(item);
            }
        }
    }
    types.sort();
    types
}

pub fn find_business(values: &[Value]) -> Option<Value> {
    let mut nodes = Vec::new();
    walk_nodes(values, &mut nodes, 0);
    let mut best: Option<Value> = None;
    let mut best_score = 0;
    for node in nodes {
        let types = type_list(node.get("@type"));
        let is_local = types
            .iter()
            .any(|t| LOCAL_TYPES.contains(&t.to_ascii_lowercase().as_str()));
        let has_address = node
            .get("address")
            .map(|a| a.is_object() || a.is_string())
            .unwrap_or(false);
        let has_name = node
            .get("name")
            .and_then(|n| n.as_str())
            .map(|n| !n.trim().is_empty())
            .unwrap_or(false);
        if !is_local && !(has_address && has_name) {
            continue;
        }
        let score = (if is_local { 3 } else { 0 })
            + (if has_address { 2 } else { 0 })
            + (if has_name { 1 } else { 0 })
            + (if node.get("telephone").is_some() { 1 } else { 0 });
        if score > best_score {
            best_score = score;
            best = Some(node.clone());
        }
    }
    best
}

fn get_str(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

fn get_num(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.parse::<f64>().ok(),
        _ => None,
    }
}

pub fn extract_nap(values: &[Value]) -> Nap {
    let mut nap = Nap::default();
    let business = match find_business(values) {
        Some(node) => node,
        None => return nap,
    };

    nap.types = type_list(business.get("@type"));
    nap.name = get_str(business.get("name"));

    if let Some(address) = business.get("address") {
        if address.is_object() {
            nap.street = get_str(address.get("streetAddress"));
            nap.locality = get_str(address.get("addressLocality"));
            nap.region = get_str(address.get("addressRegion"));
            nap.postal_code = get_str(address.get("postalCode"));
            nap.country = get_str(address.get("addressCountry"));
        } else if let Some(text) = address.as_str() {
            nap.street = Some(text.trim().to_string());
        }
    }

    nap.phone = get_str(business.get("telephone"));
    nap.price_range = get_str(business.get("priceRange"));

    if let Some(geo) = business.get("geo") {
        nap.lat = get_num(geo.get("latitude"));
        nap.lng = get_num(geo.get("longitude"));
    }

    if let Some(spec) = business.get("openingHoursSpecification") {
        let entries: Vec<&Value> = match spec {
            Value::Array(items) => items.iter().collect(),
            other => vec![other],
        };
        for entry in entries {
            if let Some(text) = entry.as_str() {
                nap.opening_hours.push(text.to_string());
                continue;
            }
            if !entry.is_object() {
                continue;
            }
            let days = type_list(entry.get("dayOfWeek"))
                .iter()
                .map(|day| day.trim_start_matches("https://schema.org/").to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let opens = get_str(entry.get("opens"));
            let closes = get_str(entry.get("closes"));
            if !days.is_empty() {
                if let (Some(opens), Some(closes)) = (opens, closes) {
                    nap.opening_hours.push(format!("{} {}-{}", days, opens, closes));
                }
            }
        }
    }
    if let Some(hours) = business.get("openingHours") {
        for item in type_list(Some(hours)) {
            nap.opening_hours.push(item);
        }
    }

    if let Some(same_as) = business.get("sameAs") {
        match same_as {
            Value::String(text) => nap.same_as.push(text.clone()),
            Value::Array(items) => {
                for item in items {
                    if let Some(text) = item.as_str() {
                        nap.same_as.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(area) = business.get("areaServed") {
        match area {
            Value::String(text) => nap.area_served.push(text.clone()),
            Value::Array(items) => {
                for item in items {
                    if let Some(text) = item.as_str() {
                        nap.area_served.push(text.to_string());
                    } else if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                        nap.area_served.push(name.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(rating) = business.get("aggregateRating") {
        nap.rating_value = get_num(rating.get("ratingValue"));
        nap.review_count = get_num(rating.get("reviewCount").or_else(|| rating.get("ratingCount")));
    }

    nap.has_map = get_str(business.get("hasMap"));
    nap
}
