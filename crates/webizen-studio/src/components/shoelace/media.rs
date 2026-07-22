//! Media components: Carousel, CarouselItem, Checkbox, ColorPicker, CopyButton, Details, Dialog, Divider

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-card`
#[component]
pub fn SlCard(hasSlotController: Option<String>, children: Element) -> Element {
    rsx! {
        sl-card {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "hasSlotController": hasSlotController.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-carousel`
#[component]
pub fn SlCarousel(
    dependencies: Option<String>,
    r#loop: Option<bool>,
    navigation: Option<bool>,
    pagination: Option<bool>,
    autoplay: Option<bool>,
    autoplayInterval: Option<f64>,
    slidesPerPage: Option<f64>,
    slidesPerMove: Option<f64>,
    orientation: Option<String>,
    mouseDragging: Option<bool>,
    scrollContainer: Option<String>,
    paginationContainer: Option<String>,
    activeSlide: Option<f64>,
    scrolling: Option<bool>,
    dragging: Option<bool>,
    autoplayController: Option<String>,
    dragStartPosition: Option<String>,
    localize: Option<String>,
    mutationObserver: Option<String>,
    pendingSlideChange: Option<bool>,
    handleMouseDrag: Option<String>,
    handleMouseDragEnd: Option<String>,
    handleSlotChange: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-carousel {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "loop": r#loop.unwrap_or_default(),
            "navigation": navigation.unwrap_or_default(),
            "pagination": pagination.unwrap_or_default(),
            "autoplay": autoplay.unwrap_or_default(),
            "autoplayInterval": autoplayInterval.unwrap_or_default(),
            "slidesPerPage": slidesPerPage.unwrap_or_default(),
            "slidesPerMove": slidesPerMove.unwrap_or_default(),
            "orientation": orientation.unwrap_or_default(),
            "mouseDragging": mouseDragging.unwrap_or_default(),
            "scrollContainer": scrollContainer.unwrap_or_default(),
            "paginationContainer": paginationContainer.unwrap_or_default(),
            "activeSlide": activeSlide.unwrap_or_default(),
            "scrolling": scrolling.unwrap_or_default(),
            "dragging": dragging.unwrap_or_default(),
            "autoplayController": autoplayController.unwrap_or_default(),
            "dragStartPosition": dragStartPosition.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "mutationObserver": mutationObserver.unwrap_or_default(),
            "pendingSlideChange": pendingSlideChange.unwrap_or_default(),
            "handleMouseDrag": handleMouseDrag.unwrap_or_default(),
            "handleMouseDragEnd": handleMouseDragEnd.unwrap_or_default(),
            "handleSlotChange": handleSlotChange.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-carousel-item`
#[component]
pub fn SlCarouselItem(children: Element) -> Element {
    rsx! {
        sl-carousel-item {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-checkbox`
#[component]
pub fn SlCheckbox(
    dependencies: Option<String>,
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    input: Option<String>,
    hasFocus: Option<bool>,
    title: Option<String>,
    name: Option<String>,
    value: Option<String>,
    size: Option<String>,
    disabled: Option<bool>,
    checked: Option<bool>,
    indeterminate: Option<bool>,
    defaultChecked: Option<bool>,
    form: Option<String>,
    required: Option<bool>,
    helpText: Option<String>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-checkbox {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "input": input.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "title": title.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "checked": checked.unwrap_or_default(),
            "indeterminate": indeterminate.unwrap_or_default(),
            "defaultChecked": defaultChecked.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "required": required.unwrap_or_default(),
            "helpText": helpText.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-color-picker`
#[component]
pub fn SlColorPicker(
    dependencies: Option<String>,
    formControlController: Option<String>,
    isSafeValue: Option<bool>,
    localize: Option<String>,
    base: Option<String>,
    input: Option<String>,
    dropdown: Option<String>,
    previewButton: Option<String>,
    trigger: Option<String>,
    hasFocus: Option<bool>,
    isDraggingGridHandle: Option<bool>,
    isEmpty: Option<bool>,
    inputValue: Option<String>,
    hue: Option<f64>,
    saturation: Option<f64>,
    brightness: Option<f64>,
    alpha: Option<f64>,
    value: Option<String>,
    defaultValue: Option<String>,
    label: Option<String>,
    format: Option<String>,
    inline: Option<bool>,
    size: Option<String>,
    noFormatToggle: Option<bool>,
    name: Option<String>,
    disabled: Option<bool>,
    hoist: Option<bool>,
    opacity: Option<bool>,
    uppercase: Option<bool>,
    swatches: Option<String>,
    form: Option<String>,
    required: Option<bool>,
    validity: Option<String>,
    validationMessage: Option<String>,
    handleFocusIn: Option<String>,
    handleFocusOut: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-color-picker {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "formControlController": formControlController.unwrap_or_default(),
            "isSafeValue": isSafeValue.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "base": base.unwrap_or_default(),
            "input": input.unwrap_or_default(),
            "dropdown": dropdown.unwrap_or_default(),
            "previewButton": previewButton.unwrap_or_default(),
            "trigger": trigger.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "isDraggingGridHandle": isDraggingGridHandle.unwrap_or_default(),
            "isEmpty": isEmpty.unwrap_or_default(),
            "inputValue": inputValue.unwrap_or_default(),
            "hue": hue.unwrap_or_default(),
            "saturation": saturation.unwrap_or_default(),
            "brightness": brightness.unwrap_or_default(),
            "alpha": alpha.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "defaultValue": defaultValue.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "format": format.unwrap_or_default(),
            "inline": inline.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "noFormatToggle": noFormatToggle.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "hoist": hoist.unwrap_or_default(),
            "opacity": opacity.unwrap_or_default(),
            "uppercase": uppercase.unwrap_or_default(),
            "swatches": swatches.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "required": required.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            "handleFocusIn": handleFocusIn.unwrap_or_default(),
            "handleFocusOut": handleFocusOut.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-copy-button`
#[component]
pub fn SlCopyButton(
    dependencies: Option<String>,
    localize: Option<String>,
    copyIcon: Option<String>,
    successIcon: Option<String>,
    errorIcon: Option<String>,
    tooltip: Option<String>,
    isCopying: Option<bool>,
    status: Option<String>,
    value: Option<String>,
    from: Option<String>,
    disabled: Option<bool>,
    copyLabel: Option<String>,
    successLabel: Option<String>,
    errorLabel: Option<String>,
    feedbackDuration: Option<f64>,
    tooltipPlacement: Option<String>,
    hoist: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-copy-button {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "copyIcon": copyIcon.unwrap_or_default(),
            "successIcon": successIcon.unwrap_or_default(),
            "errorIcon": errorIcon.unwrap_or_default(),
            "tooltip": tooltip.unwrap_or_default(),
            "isCopying": isCopying.unwrap_or_default(),
            "status": status.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "from": from.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "copyLabel": copyLabel.unwrap_or_default(),
            "successLabel": successLabel.unwrap_or_default(),
            "errorLabel": errorLabel.unwrap_or_default(),
            "feedbackDuration": feedbackDuration.unwrap_or_default(),
            "tooltipPlacement": tooltipPlacement.unwrap_or_default(),
            "hoist": hoist.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-details`
#[component]
pub fn SlDetails(
    dependencies: Option<String>,
    localize: Option<String>,
    details: Option<String>,
    header: Option<String>,
    body: Option<String>,
    expandIconSlot: Option<String>,
    detailsObserver: Option<String>,
    open: Option<bool>,
    summary: Option<String>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-details {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "details": details.unwrap_or_default(),
            "header": header.unwrap_or_default(),
            "body": body.unwrap_or_default(),
            "expandIconSlot": expandIconSlot.unwrap_or_default(),
            "detailsObserver": detailsObserver.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "summary": summary.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-dialog`
#[component]
pub fn SlDialog(
    dependencies: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    originalTrigger: Option<String>,
    modal: Option<String>,
    closeWatcher: Option<String>,
    dialog: Option<String>,
    panel: Option<String>,
    overlay: Option<String>,
    open: Option<bool>,
    label: Option<String>,
    noHeader: Option<bool>,
    handleDocumentKeyDown: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-dialog {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "originalTrigger": originalTrigger.unwrap_or_default(),
            "modal": modal.unwrap_or_default(),
            "closeWatcher": closeWatcher.unwrap_or_default(),
            "dialog": dialog.unwrap_or_default(),
            "panel": panel.unwrap_or_default(),
            "overlay": overlay.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "noHeader": noHeader.unwrap_or_default(),
            "handleDocumentKeyDown": handleDocumentKeyDown.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-divider`
#[component]
pub fn SlDivider(vertical: Option<bool>, children: Element) -> Element {
    rsx! {
        sl-divider {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "vertical": vertical.unwrap_or_default(),
            {children}
        }
    }
}
