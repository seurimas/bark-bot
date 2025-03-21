pub fn apply_tool_filters(filters: &Vec<String>, tool_name: &String) -> bool {
    for filter in filters {
        if filter.starts_with("!") {
            if tool_name.contains(&filter[1..]) {
                return false;
            }
        } else if filter.starts_with("=") {
            if tool_name.eq(&filter[1..]) {
                return true;
            }
        } else if filter.starts_with("@") {
            if tool_name.starts_with(&filter[1..]) {
                return true;
            }
        } else if filter.starts_with("*") {
            if tool_name.contains(&filter[1..]) {
                return true;
            }
        }
    }
    filters.is_empty()
}
