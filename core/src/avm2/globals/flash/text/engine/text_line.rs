use crate::avm2::activation::Activation;
use crate::avm2::error::{Error, make_error_2008};
use crate::avm2::function::FunctionArgs;
use crate::avm2::parameters::ParametersExt;
use crate::avm2::value::Value;
use crate::display_object::TDisplayObject;
use crate::fte::TextLineValidity;
use ruffle_macros::istr;
use swf::Twips;

pub fn get_text_width<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this.as_object().unwrap();
    let display_object = this.as_display_object().unwrap();
    let Some(text_line) = display_object.as_text_line() else {
        return Ok(0.0.into());
    };

    let measured_text = text_line.measure_text(activation.context);
    Ok(measured_text.0.to_pixels().into())
}

pub fn get_has_tabs<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    Ok(this.has_tabs().into())
}

pub fn get_validity<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this.as_object().unwrap();
    let display_object = this.as_display_object().unwrap();
    let Some(text_line) = display_object.as_text_line() else {
        return Ok(Value::Undefined);
    };

    let validity = match text_line.validity() {
        TextLineValidity::Valid => istr!("valid"),
        TextLineValidity::Invalid => istr!("invalid"),
        TextLineValidity::Static => istr!("static"),
        TextLineValidity::PossiblyInvalid => istr!("possiblyInvalid"),
        TextLineValidity::UserInvalid(string) => string,
    };

    Ok(validity.into())
}

pub fn set_validity<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this.as_object().unwrap();
    let display_object = this.as_display_object().unwrap();
    let Some(text_line) = display_object.as_text_line() else {
        return Ok(Value::Undefined);
    };

    let value = args.get_string_non_null(activation, 0, "validity")?;

    let previous_value = text_line.validity();
    let new_value = TextLineValidity::parse(value);

    let transition_allowed = match (previous_value, new_value) {
        (a, b) if a == b => true,
        (_, TextLineValidity::PossiblyInvalid) => false,
        (_, TextLineValidity::Static) => true,
        (TextLineValidity::Static, _) => false,
        (TextLineValidity::Invalid, _) => false,
        _ => true,
    };

    if !transition_allowed {
        return Err(make_error_2008(activation, "validity"));
    }

    text_line.set_validity(new_value, activation.gc());
    Ok(Value::Undefined)
}

pub fn get_text_block<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    let block = this.text_block_from_script();

    Ok(block.map(Value::from).unwrap_or(Value::Null))
}

pub fn get_specified_width<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    Ok(this.specified_width().into())
}

pub fn get_raw_text_length<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    Ok(this.raw_text_length().into())
}

pub fn get_text_block_begin_index<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    Ok(this.begin_index().into())
}

pub fn get_previous_line<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    Ok(this
        .previous_line()
        .and_then(|line| line.object2())
        .map(Value::from)
        .unwrap_or(Value::Null))
}

pub fn get_next_line<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();

    Ok(this
        .next_line()
        .and_then(|line| line.object2())
        .map(Value::from)
        .unwrap_or(Value::Null))
}

pub fn get_text_height<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let this = this.as_object().unwrap();
    let display_object = this.as_display_object().unwrap();
    let Some(text_line) = display_object.as_text_line() else {
        return Ok(0.0.into());
    };

    let measured_text = text_line.measure_text(activation.context);
    Ok(measured_text.1.to_pixels().into())
}

pub fn get_ascent<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let line = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();
    Ok(line
        .fallback()
        .line_metrics(0)
        .map_or(0.0, |metrics| metrics.ascent.to_pixels())
        .into())
}

pub fn get_descent<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let line = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();
    Ok(line
        .fallback()
        .line_metrics(0)
        .map_or(0.0, |metrics| metrics.descent.to_pixels())
        .into())
}

pub fn get_atom_bounds<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let line = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();
    let index = args.get_i32(0);
    let Some(bounds) = usize::try_from(index)
        .ok()
        .and_then(|index| line.atom_bounds(index))
    else {
        return Ok(Value::Null);
    };
    let rect = activation.avm2().classes().rectangle.construct(
        activation,
        &[
            bounds.x_min.to_pixels().into(),
            bounds.y_min.to_pixels().into(),
            bounds.width().to_pixels().into(),
            bounds.height().to_pixels().into(),
        ],
    )?;
    Ok(rect)
}

pub fn get_atom_index_at_point<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let line = this
        .as_object()
        .unwrap()
        .as_display_object()
        .unwrap()
        .as_text_line()
        .unwrap();
    let point = crate::prelude::Point::new(
        Twips::from_pixels(args.get_f64(0)),
        Twips::from_pixels(args.get_f64(1)),
    );
    let Some(point) = line.global_to_local(point) else {
        return Ok((-1).into());
    };
    for index in 0..line.raw_text_length() as usize {
        if line
            .atom_bounds(index)
            .is_some_and(|bounds| bounds.contains(point))
        {
            return Ok((index as i32).into());
        }
    }
    Ok((-1).into())
}
