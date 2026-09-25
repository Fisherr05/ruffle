//! `flash.desktop.Clipboard` native methods

use crate::avm2::AvmString;
use crate::avm2::activation::Activation;
use crate::avm2::error::Error;
use crate::avm2::function::FunctionArgs;
use crate::avm2::value::Value;

pub fn get_text<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let content = activation.context.ui.clipboard_content();
    Ok(AvmString::new_utf8(activation.gc(), &content).into())
}

pub fn has_text<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    Ok((!activation.context.ui.clipboard_content().is_empty()).into())
}
