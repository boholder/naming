use std::error::Error;
use std::process;

use clap::ArgMatches;

use naming_clt_lib::*;

mod app;

fn main() {
    match operate(app::app().get_matches()) {
        Ok(output) => {
            if is_atty_stdout() {
                println!("{}", output);
            } else {
                print!("{}", output);
            }
            process::exit(0);
        }
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
    };
}

fn is_atty_stdout() -> bool {
    atty::is(atty::Stream::Stdout)
}

/// A wrapper that does everything from user input to output.
fn operate(matches: ArgMatches) -> Result<String, Box<dyn Error>> {
    let text = get_text_from_input(&matches)?;
    let convertor = wrap_text_with_converter(&matches, text)?;
    output_as_string(matches, convertor)
}

fn get_text_from_input(
    matches: &ArgMatches,
) -> Result<Vec<String>, Box<dyn Error>> {
    let eof = matches.value_of("eof");
    let text = match matches.values_of_lossy("files") {
        None => {
            if is_atty_stdin() {
                return Err(
                    "naming: no input was found. Enter -h or --help for help information.".into()
                );
            } else {
                vec![read_from_std_in(eof)?]
            }
        }
        Some(files) => read_from_files(&files, eof)?,
    };
    Ok(text)
}

fn is_atty_stdin() -> bool {
    atty::is(atty::Stream::Stdin)
}

fn wrap_text_with_converter(
    matches: &ArgMatches,
    text: Vec<String>,
) -> Result<Convertor, Box<dyn Error>> {
    let option = |tag: &str| matches.values_of_lossy(tag);

    // text (String) --Captor--> words (Vec<String>)
    // --Filter--> intermediate type instances (Vec<NamingCase>)
    // --> Convertor (ready to convert itself into different format outputs)
    let convertor = Convertor::new(
        option("output"),
        Filter::new(option("filter"))?.to_naming_cases_from(
            Captor::new(option("locator"))?.capture_words(text),
        ),
    );
    Ok(convertor)
}

fn output_as_string(
    matches: ArgMatches,
    convertor: Convertor,
) -> Result<String, Box<dyn Error>> {
    let json_flag_is_passed = matches.is_present("json");
    let regex_flag_is_passed = matches.is_present("regex");

    if json_flag_is_passed && regex_flag_is_passed {
        Ok(convertor.into_regex_json())
    } else if json_flag_is_passed {
        Ok(convertor.into_json())
    } else if regex_flag_is_passed {
        Ok(convertor.into_regex())
    } else {
        Ok(convertor.into_lines())
    }
}
