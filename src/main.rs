use rig::{completion::Prompt, providers::openai};
use serde_json::Value;

#[tokio::main]
async fn main() {
    let openai_client = openai::Client::from_env();
    let openapi_fixer = openai_client
        .agent(openai::O3_MINI)
        .preamble(&"You're given a bad written openapi schema that uses examples instead of properly defining the schema. Spit the correct part without extra fuff. Just rewrite the given part not add anithing else around")
        .build();

    let schema_file = std::fs::read_to_string(&"capital.json").unwrap();

    let mut parsed_file: Value = serde_json::from_str(&schema_file).unwrap();

    let paths = parsed_file.get_mut("paths").unwrap();

    for (path, schema) in paths.as_object_mut().unwrap().iter_mut() {
        for (method, method_schema) in schema.as_object_mut().unwrap().iter_mut() {
            method_schema
                .as_object_mut()
                .unwrap()
                .remove("x-codeSamples");
            if let Some(parameters) = method_schema.get_mut("parameters") {
                parameters.as_array_mut().unwrap().retain(|param| {
                    let param_name = param
                        .as_object()
                        .unwrap()
                        .get("name")
                        .unwrap()
                        .as_str()
                        .unwrap();
                    !(param_name == "CST" || param_name == "X-SECURITY-TOKEN")
                });
            };
            if let Some(responses) = method_schema.get_mut("responses") {
                for (code, code_schema) in responses.as_object_mut().unwrap().iter_mut() {
                    let header_map = code_schema
                        .as_object_mut()
                        .unwrap()
                        .get_mut("headers")
                        .unwrap()
                        .as_object_mut()
                        .unwrap();
                    header_map.retain(|header_key, header_value| {
                        header_key == "X-SECURITY-TOKEN" || header_key == "CST"
                    });
                }
            };

            println!("processing path {} {}", path, method);
            let mut fixed_schema = openapi_fixer
                .prompt(serde_json::to_string_pretty(method_schema).unwrap())
                .await
                .unwrap();
            fixed_schema = fixed_schema.replace("```json`", "");
            fixed_schema = fixed_schema.replace("````", "");
            *method_schema = serde_json::from_str(&fixed_schema)
                .map_err(|err| {
                    eprintln!("error during parsing {:?}", fixed_schema);
                })
                .unwrap();
            println!("finished path {} {}", path, method);
        }
    }

    std::fs::write(
        &"out.json",
        serde_json::to_string_pretty(&parsed_file).unwrap(),
    )
    .unwrap();
}
