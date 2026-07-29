//! Progress/form components: ProgressBar, ProgressRing, Radio, QrCode, RadioButton, RadioGroup, Range, Rating, RelativeTime, ResizeObserver

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-progress-bar`
#[component]
pub fn SlProgressBar(
    localize: Option<String>,
    value: Option<f64>,
    indeterminate: Option<bool>,
    label: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-progress-bar {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "indeterminate": indeterminate.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-progress-ring`
#[component]
pub fn SlProgressRing(
    localize: Option<String>,
    indicator: Option<String>,
    indicatorOffset: Option<String>,
    value: Option<f64>,
    label: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-progress-ring {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            "indicator": indicator.unwrap_or_default(),
            "indicatorOffset": indicatorOffset.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-radio`
#[component]
pub fn SlRadio(
    dependencies: Option<String>,
    checked: Option<bool>,
    hasFocus: Option<bool>,
    value: Option<String>,
    size: Option<String>,
    disabled: Option<bool>,
    handleBlur: Option<String>,
    handleClick: Option<String>,
    handleFocus: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-radio {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "checked": checked.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "handleBlur": handleBlur.unwrap_or_default(),
            "handleClick": handleClick.unwrap_or_default(),
            "handleFocus": handleFocus.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-qr-code`
#[component]
pub fn SlQrCode(
    canvas: Option<String>,
    value: Option<String>,
    label: Option<String>,
    size: Option<f64>,
    fill: Option<String>,
    background: Option<String>,
    radius: Option<f64>,
    errorCorrection: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-qr-code {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "canvas": canvas.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "fill": fill.unwrap_or_default(),
            "background": background.unwrap_or_default(),
            "radius": radius.unwrap_or_default(),
            "errorCorrection": errorCorrection.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-radio-button`
#[component]
pub fn SlRadioButton(
    hasSlotController: Option<String>,
    input: Option<String>,
    hiddenInput: Option<String>,
    hasFocus: Option<bool>,
    value: Option<String>,
    disabled: Option<bool>,
    size: Option<String>,
    pill: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-radio-button {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "input": input.unwrap_or_default(),
            "hiddenInput": hiddenInput.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "pill": pill.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-radio-group`
#[component]
pub fn SlRadioGroup(
    dependencies: Option<String>,
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    customValidityMessage: Option<String>,
    validationTimeout: Option<f64>,
    defaultSlot: Option<String>,
    validationInput: Option<String>,
    hasButtonGroup: Option<bool>,
    errorMessage: Option<String>,
    defaultValue: Option<String>,
    label: Option<String>,
    helpText: Option<String>,
    name: Option<String>,
    value: Option<String>,
    size: Option<String>,
    form: Option<String>,
    required: Option<bool>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-radio-group {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "customValidityMessage": customValidityMessage.unwrap_or_default(),
            "validationTimeout": validationTimeout.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "validationInput": validationInput.unwrap_or_default(),
            "hasButtonGroup": hasButtonGroup.unwrap_or_default(),
            "errorMessage": errorMessage.unwrap_or_default(),
            "defaultValue": defaultValue.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "helpText": helpText.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "required": required.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-range`
#[component]
pub fn SlRange(
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    resizeObserver: Option<String>,
    input: Option<String>,
    output: Option<String>,
    hasFocus: Option<bool>,
    hasTooltip: Option<bool>,
    title: Option<String>,
    name: Option<String>,
    value: Option<f64>,
    label: Option<String>,
    helpText: Option<String>,
    disabled: Option<bool>,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    tooltip: Option<String>,
    tooltipFormatter: Option<String>,
    form: Option<String>,
    defaultValue: Option<f64>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-range {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "resizeObserver": resizeObserver.unwrap_or_default(),
            "input": input.unwrap_or_default(),
            "output": output.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "hasTooltip": hasTooltip.unwrap_or_default(),
            "title": title.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "helpText": helpText.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "min": min.unwrap_or_default(),
            "max": max.unwrap_or_default(),
            "step": step.unwrap_or_default(),
            "tooltip": tooltip.unwrap_or_default(),
            "tooltipFormatter": tooltipFormatter.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "defaultValue": defaultValue.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-rating`
#[component]
pub fn SlRating(
    dependencies: Option<String>,
    localize: Option<String>,
    rating: Option<String>,
    hoverValue: Option<f64>,
    isHovering: Option<bool>,
    label: Option<String>,
    value: Option<f64>,
    max: Option<f64>,
    precision: Option<f64>,
    readonly: Option<bool>,
    disabled: Option<bool>,
    getSymbol: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-rating {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "rating": rating.unwrap_or_default(),
            "hoverValue": hoverValue.unwrap_or_default(),
            "isHovering": isHovering.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "max": max.unwrap_or_default(),
            "precision": precision.unwrap_or_default(),
            "readonly": readonly.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "getSymbol": getSymbol.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-relative-time`
#[component]
pub fn SlRelativeTime(
    localize: Option<String>,
    updateTimeout: Option<f64>,
    isoTime: Option<String>,
    relativeTime: Option<String>,
    date: Option<String>,
    format: Option<String>,
    numeric: Option<String>,
    sync: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-relative-time {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            "updateTimeout": updateTimeout.unwrap_or_default(),
            "isoTime": isoTime.unwrap_or_default(),
            "relativeTime": relativeTime.unwrap_or_default(),
            "date": date.unwrap_or_default(),
            "format": format.unwrap_or_default(),
            "numeric": numeric.unwrap_or_default(),
            "sync": sync.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-resize-observer`
#[component]
pub fn SlResizeObserver(
    resizeObserver: Option<String>,
    observedElements: Option<String>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-resize-observer {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "resizeObserver": resizeObserver.unwrap_or_default(),
            "observedElements": observedElements.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            {children}
        }
    }
}
