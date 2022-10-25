use docgen::markdown::parser::{MarkdownParser, ParseOptions};
use insta::*;

#[macro_use]
extern crate indoc;

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! snapshot_test {
        ($name:ident, $input:expr, $options:expr) => {
            #[test]
            fn $name() {
                let input = indoc! {$input};
                let mut options = ParseOptions::default();
                ($options)(&mut options);

                insta::with_settings!({
                    description => stringify!($name),
                    info => &input,
                    omit_expression => true // do not include the default expression
                }, {

                    let mut parser = MarkdownParser::new(Some(options));
                    assert_debug_snapshot!(parser.parse(&input));
                });
            }
        };
    }

    snapshot_test!(
        supports_headings,
        "# My heading

    Some content

    ## Some other heading",
        |_| {}
    );

    snapshot_test!(supports_links, "\n[an link](/foo/bar)\n", |_| {});

    snapshot_test!(
        rewrite_link_root_path,
        "\n[an link](/foo/bar)\n",
        |options: &mut ParseOptions| {
            options.url_root = "/other/root".to_owned();
        }
    );

    snapshot_test!(
        does_not_rewrite_non_absolute_urls,
        "\n[an link](https://www.google.com)\n",
        |options: &mut ParseOptions| {
            options.url_root = "/other/root".to_owned();
        }
    );

    snapshot_test!(
        does_not_rewrite_relative_urls,
        "\n[a relative link](relative/link)\n",
        |options: &mut ParseOptions| {
            options.url_root = "/other/root".to_owned();
        }
    );

    snapshot_test!(
        rewrites_any_image_that_has_an_explicit_rewrite_mapping,
        "\n![an image](/assets/cat.jpg)\n",
        |options: &mut ParseOptions| {
            options.link_rewrite_rules.insert(
                "/assets/cat.jpg".to_owned(),
                "https://example.com/cat.jpg".to_owned(),
            );
        }
    );

    snapshot_test!(
        rewrites_any_link_that_has_an_explicit_rewrite_mapping,
        "\n[an document](/assets/plans.pdf)\n",
        |options: &mut ParseOptions| {
            options.link_rewrite_rules.insert(
                "/assets/plans.pdf".to_owned(),
                "https://example.com/plans.pdf".to_owned(),
            );
        }
    );

    snapshot_test!(
        appends_parameters_to_the_end_of_urls,
        "\n[an link](relative/link)\n",
        |options: &mut ParseOptions| {
            options
                .url_params
                .push(("base".to_owned(), "123".to_owned()));
        }
    );

    snapshot_test!(
        appends_multiple_parameters_to_the_end_of_urls,
        "\n[an link](relative/link)\n",
        |options: &mut ParseOptions| {
            options
                .url_params
                .push(("bases".to_owned(), "are".to_owned()));
            options
                .url_params
                .push(("belong".to_owned(), "tous".to_owned()));
        }
    );

    snapshot_test!(
        appends_multiple_parameters_to_the_end_of_absolute_urls,
        "\n[an link](/absolute/link)\n",
        |options: &mut ParseOptions| {
            options
                .url_params
                .push(("bases".to_owned(), "are".to_owned()));
            options
                .url_params
                .push(("belong".to_owned(), "tous".to_owned()));
        }
    );

    snapshot_test!(
        does_not_append_params_to_urls_with_a_specific_domain,
        "\n[an link](http://www.example.com/)\n",
        |options: &mut ParseOptions| {
            options
                .url_params
                .push(("bases".to_owned(), "are".to_owned()));
            options
                .url_params
                .push(("belong".to_owned(), "tous".to_owned()));
        }
    );

    snapshot_test!(
        sanitizes_input,
        "<script>
        alert('I break you');
        </script>",
        |_| {}
    );

    snapshot_test!(
        allows_mermaid_blocks,
        "```mermaid
        graph TD;
            A-->B;
            A-->C;
        ```",
        |_| {}
    );

    snapshot_test!(
        allows_code_blocks,
        "```ruby
        1 + 1
        something else
        something else too
        another something else
        ```",
        |_| {}
    );

    snapshot_test!(
        gathers_a_list_of_links_on_the_page,
        "[foo](/bar)
        [Example](https://www.example.com)
        [Example 2](https://www.example2.com)",
        |_| {}
    );

    snapshot_test!(
        gathers_the_internal_text_of_a_link,
        "[**BOLD**](/bar)
        [![AltText](/src/foo)](/bar)
        ## [AnHeader](/bar)",
        |_| {}
    );

    snapshot_test!(detects_emojis, "I am :grinning:.", |_| {});

    snapshot_test!(detects_emojis_in_links, "[:grinning:](/foo)", |_| {});

    snapshot_test!(
        leaves_the_emoji_identifier_alone_if_it_is_not_recognised,
        "Look at this :idonotexist:",
        |_| {}
    );

    snapshot_test!(
        ignores_identifiers_that_do_not_end,
        "Look at this :stop",
        |_| {}
    );

    snapshot_test!(
        ignores_identifiers_that_do_not_end_with_whitespace,
        "Look at this :stop MORE",
        |_| {}
    );

    snapshot_test!(
        supports_tabgroups,
        "# [_Tab1_](#/tab/id1/condition1)
        Foo
        # [Tab2](#/tab/id2)
        Bar

        ---",
        |_| {}
    );

    snapshot_test!(
        supports_multiple_tabgroups,
        "# [_Tab1_](#/tab/id1/condition1)
        Foo
        # [Tab2](#/tab/id2)
        Bar

        ---
        
        # [_Tab1_](#/tab/id1/condition1)
        Foo
        # [Tab2](#/tab/id2)
        Bar

        ---
        
        After content",
        |_| {}
    );

    snapshot_test!(
        supports_content_around_tabgroups,
        "content before

        # [_Tab1_](#/tab/id1/condition1)
        Foo
        # [Tab2](#/tab/id2)
        Bar

        ---
        
        content after",
        |_| {}
    );

    snapshot_test!(
        last_tabgroup_in_doc_does_not_require_ending_rule,
        "content before

        # [_Tab1_](#/tab/id1/condition1)
        Foo
        # [Tab2](#/tab/id2)
        Bar",
        |_| {}
    );

    snapshot_test!(
        supports_headings_inside_tabgroups,
        "content before

        # [_Tab1_](#/tab/id1/condition1)
        
        ## Heading inside tab 1

        some content

        # [Tab2](#/tab/id2)
        Bar

        ## Heading inside tab 2

        ---
        
        content after",
        |_| {}
    );

    snapshot_test!(
        callout_title_is_optional,
        "> warning
        > 
        > The content",
        |_| {}
    );

    snapshot_test!(
        callouts_can_have_stuff_after_it,
        "> warning An Note
        > 
        > The content

        Moar
        ",
        |_| {}
    );

    snapshot_test!(
        callouts_can_contain_images,
        "> info An Info
        >
        > ![an pic](/cat.jpg)",
        |_| {}
    );

    snapshot_test!(
        blockquotes_are_not_confused_with_callouts,
        "> I am here!",
        |_| {}
    );

    snapshot_test!(
        supports_github_style_markdown_checkboxes,
        "
        * [ ] Incomplete
        * [x] Complete
        ",
        |_| {}
    );

    snapshot_test!(
        does_not_allow_random_forms,
        "<form>
          <label for=\"ufname\">First name:</label><br>
          <input type=\"text\" id=\"fname\" name=\"fname\"><br>
          <label for=\"lname\">Last name:</label><br>
          <input type=\"text\" id=\"lname\" name=\"lname\">
        </form>",
        |_| {}
    );

    snapshot_test!(
        it_detects_math_blocks,
        "```math
        x^2 - 5x + 6 = 0 \\
        (x-2)(x-3)=0 \\
        \\textrm{then either }x=2 \\,or\\,x=3
        ```
        ",
        |_| {}
    );

    snapshot_test!(
        inline_code_in_headings_is_included_in_heading_titles,
        "# Foo `bar` baz
        ",
        |_| {}
    );

    // snapshot_test!(
    //     supports_markdown_source_embeds,
    // "I was working but I couldn't.

    // # Heading in main file

    // ![](/embed/file.md)

    // And some more content here.",
    //     |options: &mut ParseOptions| {
    // options.resolve_embeds = Some(Box::new(|url: String| match url.as_str() {
    //     "/embed/file.md" => Some("# Heading from another file".to_string()),
    //     _ => None,
    // }));
    //     }
    // );

    // #[test]
    // fn supports_markdown_source_embeds() {
    //     let input = indoc!(
    //         "I was working but I couldn't.

    //     # Heading in main file

    //     [embed ./html/hello.md](./html/hello.md)

    //     And some more content here."
    //     );

    //     insta::with_settings!({
    //         description => "Supports callout blocks",
    //         info => &input,
    //         omit_expression => true // do not include the default expression
    //     }, {
    //         let mut parser = MarkdownParser::new(None);
    //         assert_debug_snapshot!(parser.parse(&input));
    //     });
    // }

    #[test]
    fn supports_callout_blocks() {
        let binding = ["info", "notice", "success", "warn", "warning", "error"]
            .map(|kind| {
                formatdoc! {"> {} An Note
                >
                > The content
                >
                > More content", kind}
            })
            .join("\n\n---\n\n");

        let input = binding.as_str();

        insta::with_settings!({
            description => "Supports callout blocks",
            info => &input,
            omit_expression => true // do not include the default expression
        }, {
            let mut parser = MarkdownParser::new(None);
            assert_debug_snapshot!(parser.parse(&input));
        });
    }
}
