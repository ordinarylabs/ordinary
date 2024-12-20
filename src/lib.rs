use sailfish::TemplateSimple;
use serde::Serialize;

#[derive(TemplateSimple)]
#[template(path = "index.html.stpl")]
struct IndexHtmlTemplate<'a> {
    messages: Vec<&'a str>,
    title: &'a str,
}

#[derive(Serialize)]
struct ComplexMessage<'a> {
    text: &'a str,
    other: u8,
}

#[derive(TemplateSimple)]
#[template(path = "index.json.stpl")]
struct IndexJsonTemplate<'a> {
    messages: Vec<ComplexMessage<'a>>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn html_template_test() {
        let tpl = IndexHtmlTemplate {
            messages: vec!["foo", "bar"],
            title: "sup",
        };

        let mut buffer = Buffer::with_capacity(200);
        println!("{}", buffer.len());
        tpl.render_once_to(&mut buffer).unwrap();
        println!("{}", buffer.len());

        println!("{}", buffer.as_str())
    }

    #[test]
    fn json_template_test() {
        let tpl = IndexJsonTemplate {
            messages: vec![
                ComplexMessage {
                    text: "foo",
                    other: 0,
                },
                ComplexMessage {
                    text: "bar",
                    other: 1,
                },
            ],
        };

        let mut buffer = Buffer::with_capacity(200);
        println!("{}", buffer.len());
        tpl.render_once_to(&mut buffer).unwrap();
        println!("{}", buffer.len());

        println!("{}", buffer.as_str())
    }
}
