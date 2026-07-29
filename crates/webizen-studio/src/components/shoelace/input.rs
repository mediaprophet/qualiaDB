//! Input components: Input, Menu, MenuLabel, MutationObserver, MenuItem, Option, Popup

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-image-comparer`
#[component]
pub fn SlImageComparer(
    scopedElement: Option<String>,
    localize: Option<String>,
    base: Option<String>,
    handle: Option<String>,
    position: Option<f64>,
    children: Element,
) -> Element {
    rsx! {
        sl-image-comparer {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "scopedElement": scopedElement.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "base": base.unwrap_or_default(),
            "handle": handle.unwrap_or_default(),
            "position": position.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-input`
#[component]
pub fn SlInput(
    dependencies: Option<String>,
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    input: Option<String>,
    hasFocus: Option<bool>,
    title: Option<String>,
    r#type: Option<String>,
    name: Option<String>,
    value: Option<String>,
    defaultValue: Option<String>,
    size: Option<String>,
    filled: Option<bool>,
    pill: Option<bool>,
    label: Option<String>,
    helpText: Option<String>,
    clearable: Option<bool>,
    disabled: Option<bool>,
    placeholder: Option<String>,
    readonly: Option<bool>,
    passwordToggle: Option<bool>,
    passwordVisible: Option<bool>,
    noSpinButtons: Option<bool>,
    form: Option<String>,
    required: Option<bool>,
    pattern: Option<String>,
    minlength: Option<f64>,
    maxlength: Option<f64>,
    min: Option<String>,
    max: Option<String>,
    step: Option<String>,
    autocapitalize: Option<String>,
    autocorrect: Option<String>,
    autocomplete: Option<String>,
    autofocus: Option<bool>,
    enterkeyhint: Option<String>,
    spellcheck: Option<bool>,
    inputmode: Option<String>,
    valueAsDate: Option<String>,
    valueAsNumber: Option<String>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-input {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "input": input.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "title": title.unwrap_or_default(),
            "type": r#type.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "defaultValue": defaultValue.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "filled": filled.unwrap_or_default(),
            "pill": pill.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "helpText": helpText.unwrap_or_default(),
            "clearable": clearable.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "placeholder": placeholder.unwrap_or_default(),
            "readonly": readonly.unwrap_or_default(),
            "passwordToggle": passwordToggle.unwrap_or_default(),
            "passwordVisible": passwordVisible.unwrap_or_default(),
            "noSpinButtons": noSpinButtons.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "required": required.unwrap_or_default(),
            "pattern": pattern.unwrap_or_default(),
            "minlength": minlength.unwrap_or_default(),
            "maxlength": maxlength.unwrap_or_default(),
            "min": min.unwrap_or_default(),
            "max": max.unwrap_or_default(),
            "step": step.unwrap_or_default(),
            "autocapitalize": autocapitalize.unwrap_or_default(),
            "autocorrect": autocorrect.unwrap_or_default(),
            "autocomplete": autocomplete.unwrap_or_default(),
            "autofocus": autofocus.unwrap_or_default(),
            "enterkeyhint": enterkeyhint.unwrap_or_default(),
            "spellcheck": spellcheck.unwrap_or_default(),
            "inputmode": inputmode.unwrap_or_default(),
            "valueAsDate": valueAsDate.unwrap_or_default(),
            "valueAsNumber": valueAsNumber.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-menu`
#[component]
pub fn SlMenu(defaultSlot: Option<String>, children: Element) -> Element {
    rsx! {
        sl-menu {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "defaultSlot": defaultSlot.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-menu-label`
#[component]
pub fn SlMenuLabel(children: Element) -> Element {
    rsx! {
        sl-menu-label {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-mutation-observer`
#[component]
pub fn SlMutationObserver(
    mutationObserver: Option<String>,
    attr: Option<String>,
    attrOldValue: Option<bool>,
    charData: Option<bool>,
    charDataOldValue: Option<bool>,
    childList: Option<bool>,
    disabled: Option<bool>,
    handleMutation: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-mutation-observer {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "mutationObserver": mutationObserver.unwrap_or_default(),
            "attr": attr.unwrap_or_default(),
            "attrOldValue": attrOldValue.unwrap_or_default(),
            "charData": charData.unwrap_or_default(),
            "charDataOldValue": charDataOldValue.unwrap_or_default(),
            "childList": childList.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "handleMutation": handleMutation.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-menu-item`
#[component]
pub fn SlMenuItem(
    dependencies: Option<String>,
    cachedTextLabel: Option<String>,
    localize: Option<String>,
    defaultSlot: Option<String>,
    menuItem: Option<String>,
    r#type: Option<String>,
    checked: Option<bool>,
    value: Option<String>,
    loading: Option<bool>,
    disabled: Option<bool>,
    hasSlotController: Option<String>,
    submenuController: Option<String>,
    handleHostClick: Option<String>,
    handleMouseOver: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-menu-item {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "cachedTextLabel": cachedTextLabel.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "menuItem": menuItem.unwrap_or_default(),
            "type": r#type.unwrap_or_default(),
            "checked": checked.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "loading": loading.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "submenuController": submenuController.unwrap_or_default(),
            "handleHostClick": handleHostClick.unwrap_or_default(),
            "handleMouseOver": handleMouseOver.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-option`
#[component]
pub fn SlOption(
    dependencies: Option<String>,
    localize: Option<String>,
    isInitialized: Option<bool>,
    defaultSlot: Option<String>,
    current: Option<bool>,
    selected: Option<bool>,
    hasHover: Option<bool>,
    value: Option<String>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-option {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "isInitialized": isInitialized.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "current": current.unwrap_or_default(),
            "selected": selected.unwrap_or_default(),
            "hasHover": hasHover.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-popup`
#[component]
pub fn SlPopup(
    anchorEl: Option<String>,
    cleanup: Option<String>,
    localize: Option<String>,
    popup: Option<String>,
    arrowEl: Option<String>,
    anchor: Option<String>,
    active: Option<bool>,
    placement: Option<String>,
    strategy: Option<String>,
    distance: Option<f64>,
    skidding: Option<f64>,
    arrow: Option<bool>,
    arrowPlacement: Option<String>,
    arrowPadding: Option<f64>,
    flip: Option<bool>,
    flipFallbackPlacements: Option<String>,
    flipFallbackStrategy: Option<String>,
    flipBoundary: Option<String>,
    flipPadding: Option<f64>,
    shift: Option<bool>,
    shiftBoundary: Option<String>,
    shiftPadding: Option<f64>,
    autoSize: Option<String>,
    sync: Option<String>,
    autoSizeBoundary: Option<String>,
    autoSizePadding: Option<f64>,
    hoverBridge: Option<bool>,
    updateHoverBridge: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-popup {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "anchorEl": anchorEl.unwrap_or_default(),
            "cleanup": cleanup.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "popup": popup.unwrap_or_default(),
            "arrowEl": arrowEl.unwrap_or_default(),
            "anchor": anchor.unwrap_or_default(),
            "active": active.unwrap_or_default(),
            "placement": placement.unwrap_or_default(),
            "strategy": strategy.unwrap_or_default(),
            "distance": distance.unwrap_or_default(),
            "skidding": skidding.unwrap_or_default(),
            "arrow": arrow.unwrap_or_default(),
            "arrowPlacement": arrowPlacement.unwrap_or_default(),
            "arrowPadding": arrowPadding.unwrap_or_default(),
            "flip": flip.unwrap_or_default(),
            "flipFallbackPlacements": flipFallbackPlacements.unwrap_or_default(),
            "flipFallbackStrategy": flipFallbackStrategy.unwrap_or_default(),
            "flipBoundary": flipBoundary.unwrap_or_default(),
            "flipPadding": flipPadding.unwrap_or_default(),
            "shift": shift.unwrap_or_default(),
            "shiftBoundary": shiftBoundary.unwrap_or_default(),
            "shiftPadding": shiftPadding.unwrap_or_default(),
            "autoSize": autoSize.unwrap_or_default(),
            "sync": sync.unwrap_or_default(),
            "autoSizeBoundary": autoSizeBoundary.unwrap_or_default(),
            "autoSizePadding": autoSizePadding.unwrap_or_default(),
            "hoverBridge": hoverBridge.unwrap_or_default(),
            "updateHoverBridge": updateHoverBridge.unwrap_or_default(),
            {children}
        }
    }
}
