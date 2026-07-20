use placard_css::{Combinator, Value};

fn format_value(value: &Value) -> String {
    match value {
        Value::Keyword(k) => format!("keyword({k})"),
        Value::Length(l) => format!("length({l}px)"),
        Value::Percent(p) => format!("percent({p}%)"),
        Value::Em(e) => format!("em({e}em)"),
        Value::Rem(r) => format!("rem({r}rem)"),
        Value::Fr(f) => format!("fr({f}fr)"),
        Value::Color(c) => format!("color(rgba({},{},{},{}))", c.r, c.g, c.b, c.a),
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(format_value).collect();
            format!("list({})", parts.join(", "))
        }
    }
}

fn main() {
    let css = r#"
        /* a comment that should be skipped */
        div.badge#main {
            color: #ff0000;
            background-color: rgba(0, 128, 255, 0.5);
        }

        div > p.label {
            font-size: 12px;
            margin: 0;
        }

        h1, h2, .title {
            color: navy;
            display: block;
        }

        ul li a {
            color: blue;
        }
    "#;

    let stylesheet = placard_css::parse(css);

    for rule in &stylesheet.rules {
        for selector in &rule.selectors {
            print!("selector:");
            for (i, part) in selector.parts.iter().enumerate() {
                if i > 0 {
                    let sep = match selector.combinators[i - 1] {
                        Combinator::Child => " > ",
                        Combinator::Descendant => " ",
                        Combinator::Adjacent => " + ",
                        Combinator::General => " ~ ",
                    };
                    print!("{sep}");
                }
                if let Some(tag) = &part.tag {
                    print!("{tag}");
                }
                if let Some(id) = &part.id {
                    print!("#{id}");
                }
                for class in &part.classes {
                    print!(".{class}");
                }
            }
            println!();
        }
        for decl in &rule.declarations {
            let value_str = format_value(&decl.value);
            println!("  {}: {value_str}", decl.property);
        }
        println!();
    }
}
