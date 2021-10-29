use std::collections::HashMap;

use naming_lib::{self as naming, NamingCase};

/// Answer user's `--filter` option,
/// ignore captured words that user not indicates in `--filter` option,
/// and convert words to NamingCase instances.
pub struct Filter<'a> {
    options: Vec<&'a str>,
}

impl<'a> Filter<'a> {
    pub fn new(options: &'a Vec<&str>) -> Result<Filter<'a>, &'static str> {
        if Filter::has_hungarian_camel_conflict(&options) {
            return Err("In option \"--filter\", at most one of the two, \
            hungarian notation (h) and camel case (c) can appear.");
        }
        Ok(Filter { options: options.to_vec() })
    }

    fn has_hungarian_camel_conflict(options: &Vec<&str>) -> bool {
        options.contains(&"h") && options.contains(&"c")
    }

    /// Not only transform String to NamingCase,
    /// but also apply given filter on result vector.
    pub fn to_naming_cases_from(&self, words: Vec<String>) -> Vec<NamingCase> {
        let words = self.filter_words_with_options(words);

        // if user wants to treat camel case words as hungarian notation format.
        let required_hungarian = self.options.contains(&"h");
        words.iter()
            .map(|word| {
                if required_hungarian && naming::is_camel(word) {
                    naming::from_hungarian_notation(word)
                } else {
                    naming::which_case(word)
                }
            }).collect()
    }

    const PREDICATES: [(&'static str, fn(&str) -> bool); 6] =
        [("S", naming::is_screaming_snake),
            ("s", naming::is_snake),
            ("k", naming::is_kebab),
            ("c", naming::is_camel),
            ("h", naming::is_camel),
            ("p", naming::is_pascal)];

    fn filter_words_with_options(&self, words: Vec<String>) -> Vec<String> {
        let predicates: Vec<fn(&str) -> bool> = Filter::PREDICATES.iter()
            .filter(|(opt, _)| self.options.contains(opt))
            .map(|(_, f)| *f).collect();

        words.into_iter()
            .filter(|word| {
                // check if word's format belongs to one of predicates
                predicates.iter()
                    .map(|f| f(word))
                    .reduce(|a, b| a || b)
                    .unwrap()
            }).collect()
    }
}

pub struct Convertor<'a> {
    options: Vec<&'a str>,
    cases: Vec<NamingCase>,
}

impl<'a> Convertor<'a> {
    pub fn new(options: &'a Vec<&str>, cases: Vec<NamingCase>) -> Convertor<'a> {
        Convertor { options: options.to_vec(), cases }
    }

    fn select_mappers_base_on_options(&self, mappers: &HashMap<&'static str, fn(&NamingCase) -> String>)
                                      -> Box<[fn(&NamingCase) -> String]> {
        // let the order of mappers to be same as
        // the order of options in vector.
        self.options.iter()
            .map(|option| *mappers.get(option).unwrap())
            .collect()
    }

    /// Normal output format, each line represent a captures in input text.
    ///
    /// Output looks like:
    /// \<origin string of capture1\> \<first target naming case\> \<second format\> ...
    /// \<origin string of capture2\> \<first target naming case\> \<second format\> ...
    /// ...
    pub fn into_lines(self) -> String {
        let mappers = self.select_mappers_base_on_options(
            &super::DIRECT_MAPPERS);

        self.cases.into_iter()
            .map(|case| {
                // keep the origin string as the first word.
                let mut line = case.to_string();
                line.push(' ');

                // append target words behind.
                line.push_str(
                    &mappers.iter()
                        .map(|f| f(&case))
                        .collect::<Vec<String>>().join(" "));
                line
                // each word in input -> one line of result in output
            }).collect::<Vec<String>>().join("\n")
    }

    /// Output in this format when user enters `--json` option,
    /// each array element in "result" field represent a captures in input text.
    ///
    /// Output looks like:
    /// {"result":[{"origin":\<capture1\>,\<first target format\>:\<converted string\>,...},...]}
    pub fn into_json(self) -> String {
        let mappers = self.select_mappers_base_on_options(
            &super::JSON_MAPPERS);

        let mut result = String::from(r#"{"result":["#);

        // string "{...},{...},..." for put into json array
        let json_array_fields = self.cases.into_iter()
            .map(|case| {
                let mut line = r#"{"origin":""#.to_string() + &case.to_string() + "\",";

                line.push_str(
                    &mappers.iter()
                        .map(|f| f(&case))
                        .collect::<Vec<String>>().join(","));

                line.push_str("}");
                // "{"origin":"a_a","camel":"aA",...}"
                line
            }).collect::<Vec<String>>().join(",");

        result.push_str(&json_array_fields);
        result.push_str("]}");
        // "{"result":[{...},{...},...]}"
        result
    }

    /// Output in this format when user enters `--regex` option,
    /// each line represent a captures in input text.
    ///
    /// Output looks like:
    /// \<origin string of capture1\> \<target formats mixed OR regex (e.g. "aA|a_a|a-a")\>
    /// \<origin string of capture2\> \<target formats mixed OR regex\>
    /// ...
    pub fn into_regex(self) -> String {
        let mappers = self.select_mappers_base_on_options(
            &super::DIRECT_MAPPERS);

        self.cases.into_iter()
            .map(|case| {
                // keep the origin string as the first word.
                let mut line = case.to_string();
                line.push(' ');

                // join target formats into one regex string with "|"
                line.push_str(
                    &mappers.iter()
                        .map(|f| f(&case))
                        .collect::<Vec<String>>().join("|"));
                line
            }).collect::<Vec<String>>().join("\n")
    }

    /// Output in this format when user enters both `--regex` and `-json` options,
    /// each array element in "result" field represent a captures in input text.
    ///
    /// Output looks like:
    /// {"result":[{"origin":\<capture1\>,"regex":\<mixed regex string\>},{...},...]}
    pub fn into_regex_json(self) -> String {
        let mappers = self.select_mappers_base_on_options(
            &super::DIRECT_MAPPERS);

        let mut result = String::from(r#"{"result":["#);

        // string "{...},{...},..." for put into json array
        let json_array_fields = self.cases.into_iter()
            .map(|case| {
                let mut line = r#"{"origin":""#.to_string() + &case.to_string()
                    + r#"","regex":""#;

                // concat target formats into an OR regex
                line.push_str(
                    &mappers.iter()
                        .map(|f| f(&case))
                        .collect::<Vec<String>>().join("|"));

                line.push_str("\"}");
                // "{"origin":"a_a","regex":"aA|a_a|AA"}"
                line
            }).collect::<Vec<String>>().join(",");

        result.push_str(&json_array_fields);
        result.push_str("]}");
        // "{"result":[{...},{...},...]}"
        result
    }
}

#[cfg(test)]
mod filter_tests {
    use naming_lib::NamingCase;

    use super::Filter;

    #[test]
    fn find_hungarian_camel_conflict() {
        assert!(Filter::has_hungarian_camel_conflict(&vec!["c", "h"]));
    }

    #[test]
    fn filter_words_with_option() {
        let options = &vec!["S", "s", "k", "c", "p"];
        let filter = Filter::new(options).unwrap();
        let words: Vec<String> = vec!["SCREAMING_SNAKE", "snake_case",
                                      "kebab-case", "camelCase", "PascalCase",
                                      "-invalid_"].into_iter()
            .map(|s| s.to_string()).collect();

        let mut expect = words.clone();
        // remove the invalid word at tail
        expect.pop();
        let actual = Filter::filter_words_with_options(&filter, words);
        assert_eq!(actual, expect);
    }

    #[test]
    fn convert_words_as_hungarian_notation() {
        let options = vec!["h"];
        let words: Vec<String> = vec!["intPageSize".to_string()];

        let actual = Filter::new(&options).unwrap()
            .to_naming_cases_from(words);
        assert_eq!(actual, vec![NamingCase::Pascal("PageSize".to_string())]);
    }

    #[test]
    fn convert_words_to_naming_cases() {
        let options = vec!["S", "s", "k", "c", "p"];
        let words: Vec<String> = vec!["SCREAMING_SNAKE", "snake_case",
                                      "kebab-case", "camelCase", "PascalCase",
                                      "-invalid_"].into_iter()
            .map(|s| s.to_string()).collect();

        let actual = Filter::new(&options).unwrap()
            .to_naming_cases_from(words);

        let expect = vec![
            NamingCase::ScreamingSnake("SCREAMING_SNAKE".to_string()),
            NamingCase::Snake("snake_case".to_string()),
            NamingCase::Kebab("kebab-case".to_string()),
            NamingCase::Camel("camelCase".to_string()),
            NamingCase::Pascal("PascalCase".to_string())];

        assert_eq!(actual, expect);
    }
}

#[cfg(test)]
mod convertor_tests {
    use naming_lib as naming;

    use super::Convertor;

    #[test]
    fn output_to_lines() {
        let options = vec!["S", "s", "k", "c", "p"];

        let words = vec!["SCREAMING_SNAKE", "snake_case",
                         "kebab-case", "camelCase", "PascalCase"];
        let cases = words.into_iter()
            .map(|word| naming::which_case(word)).collect();

        let actual = Convertor::new(&options, cases).into_lines();

        let expect = "\
SCREAMING_SNAKE SCREAMING_SNAKE screaming_snake screaming-snake screamingSnake ScreamingSnake
snake_case SNAKE_CASE snake_case snake-case snakeCase SnakeCase
kebab-case KEBAB_CASE kebab_case kebab-case kebabCase KebabCase
camelCase CAMEL_CASE camel_case camel-case camelCase CamelCase
PascalCase PASCAL_CASE pascal_case pascal-case pascalCase PascalCase";

        assert_eq!(actual.as_str(), expect);
    }

    #[test]
    fn output_bases_on_options_order() {
        let options = vec!["p", "c", "s", "k", "S"];
        let cases = vec!["a_a"].into_iter()
            .map(|word| naming::which_case(word)).collect();

        let actual = Convertor::new(&options, cases).into_lines();
        assert_eq!(actual.as_str(), "a_a AA aA a_a a-a A_A");
    }

    #[test]
    fn output_to_json() {
        let options = vec!["S", "s", "k", "c", "p"];
        let words = vec!["snake_case", "kebab-case"];
        let cases = words.into_iter()
            .map(|word| naming::which_case(word)).collect();

        let actual = Convertor::new(&options, cases).into_json();

        let expect = concat!(
        r#"{"result":[{"origin":"snake_case","screaming_snake":"SNAKE_CASE","snake":"snake_case","#,
        r#""kebab":"snake-case","camel":"snakeCase","pascal":"SnakeCase"},"#,
        r#"{"origin":"kebab-case","screaming_snake":"KEBAB_CASE","snake":"kebab_case","#,
        r#""kebab":"kebab-case","camel":"kebabCase","pascal":"KebabCase"}]}"#
        );

        assert_eq!(actual.as_str(), expect);
    }

    #[test]
    fn output_to_regex() {
        let options = vec!["S", "s", "k", "c", "p"];

        let words = vec!["SCREAMING_SNAKE", "snake_case"];
        let cases = words.into_iter()
            .map(|word| naming::which_case(word)).collect();

        let actual = Convertor::new(&options, cases).into_regex();

        let expect = "\
SCREAMING_SNAKE SCREAMING_SNAKE|screaming_snake|screaming-snake|screamingSnake|ScreamingSnake
snake_case SNAKE_CASE|snake_case|snake-case|snakeCase|SnakeCase";

        assert_eq!(actual.as_str(), expect);
    }

    #[test]
    fn output_to_regex_json() {
        let options = vec!["S", "s", "k", "c", "p"];
        let words = vec!["SCREAMING_SNAKE", "snake_case"];
        let cases = words.into_iter()
            .map(|word| naming::which_case(word)).collect();

        let actual = Convertor::new(&options, cases).into_regex_json();

        let expect = concat!(
        r#"{"result":[{"origin":"SCREAMING_SNAKE","#,
        r#""regex":"SCREAMING_SNAKE|screaming_snake|screaming-snake|screamingSnake|ScreamingSnake"},"#,
        r#"{"origin":"snake_case","#,
        r#""regex":"SNAKE_CASE|snake_case|snake-case|snakeCase|SnakeCase"}]}"#
        );

        assert_eq!(actual.as_str(), expect);
    }
}