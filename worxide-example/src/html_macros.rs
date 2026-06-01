#![allow(unused)]

use {dominator::DomBuilder, web_sys::*};

macro_rules! new_el_fns {
    ( $( $el:ident => $tag:ty ),* $(,)? ) => {
    $(
        pub fn $el() -> DomBuilder<$tag> {
            DomBuilder::<$tag>::new_html(&stringify!($el).replace("html_", ""))
        }
    )*
    };
}

new_el_fns! {
html_a => HtmlAnchorElement,
html_abbr => HtmlElement,
html_aside => HtmlElement,
html_br => HtmlElement,
html_button => HtmlButtonElement,
html_canvas => HtmlCanvasElement,
html_code => HtmlElement,
html_details => HtmlDetailsElement,
html_div => HtmlDivElement,
html_em => HtmlElement,
html_embed => HtmlEmbedElement,
html_figure => HtmlElement,
html_footer => HtmlElement,
html_form => HtmlFormElement,
html_header => HtmlElement,
html_h1 => HtmlHeadingElement,
html_h2 => HtmlHeadingElement,
html_h3 => HtmlHeadingElement,
html_h4 => HtmlHeadingElement,
html_h5 => HtmlHeadingElement,
html_h6 => HtmlHeadingElement,
html_hr => HtmlElement,
html_i => HtmlElement,
html_iframe => HtmlIFrameElement,
html_img => HtmlImageElement,
html_input => HtmlInputElement,
html_kbd => HtmlElement,
html_label => HtmlElement,
html_li => HtmlElement,
html_main => HtmlElement,
html_nav => HtmlElement,
html_object => HtmlObjectElement,
html_option => HtmlOptionElement,
html_p => HtmlParagraphElement,
html_pre => HtmlElement,
html_progress => HtmlProgressElement,
html_section => HtmlElement,
html_select => HtmlSelectElement,
html_span => HtmlSpanElement,
html_strong => HtmlElement,
html_sub => HtmlElement,
html_sup => HtmlElement,
html_summary => HtmlElement,
html_table => HtmlTableElement,
html_tbody => HtmlTableSectionElement,
html_td => HtmlTableCellElement,
html_textarea => HtmlTextAreaElement,
html_tfoot => HtmlTableSectionElement,
html_th => HtmlTableCellElement,
html_thead => HtmlTableSectionElement,
html_tr => HtmlTableRowElement,
html_ul => HtmlElement,
html_ol => HtmlElement,
html_wrapper => HtmlElement,
}
