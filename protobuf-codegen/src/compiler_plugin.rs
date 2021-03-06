use std::io::stdin;
use std::io::stdout;
use std::str;
use protobuf::plugin::*;
use protobuf::parse_from_reader;
use protobuf::Message;
use protobuf::descriptor::FileDescriptorProto;

pub struct GenRequest<'a> {
    pub file_descriptors: &'a [FileDescriptorProto],
    pub files_to_generate: &'a [String],
    pub parameter: &'a str,
}

pub struct GenResult {
    pub name: String,
    pub content: Vec<u8>,
}

pub fn plugin_main<F>(gen: F)
    where F : Fn(&GenRequest) -> Vec<GenResult>
{
    let req = parse_from_reader::<CodeGeneratorRequest>(&mut stdin()).unwrap();
    let result = gen(&GenRequest {
        file_descriptors: &req.proto_file,
        files_to_generate: &req.file_to_generate,
        parameter: req.get_parameter(),
    });
    let mut resp = CodeGeneratorResponse::new();
    resp.file =
        result
            .iter()
            .map(|file| {
                let mut r = CodeGeneratorResponse_File::new();
                r.set_name(file.name.to_string());
                r.set_content(str::from_utf8(file.content.as_ref()).unwrap().to_string());
                r
            })
            .collect();
    resp.write_to_writer(&mut stdout()).unwrap();
}
