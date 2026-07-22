//! Overlay components: Drawer, Dropdown, FormatBytes, FormatDate, FormatNumber, Icon, Include, IconButton, ImageComparer

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-drawer`
#[component]
pub fn SlDrawer(
    dependencies: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    originalTrigger: Option<String>,
    modal: Option<String>,
    closeWatcher: Option<String>,
    drawer: Option<String>,
    panel: Option<String>,
    overlay: Option<String>,
    open: Option<bool>,
    label: Option<String>,
    placement: Option<String>,
    contained: Option<bool>,
    noHeader: Option<bool>,
    handleDocumentKeyDown: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-drawer {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "originalTrigger": originalTrigger.unwrap_or_default(),
            "modal": modal.unwrap_or_default(),
            "closeWatcher": closeWatcher.unwrap_or_default(),
            "drawer": drawer.unwrap_or_default(),
            "panel": panel.unwrap_or_default(),
            "overlay": overlay.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "placement": placement.unwrap_or_default(),
            "contained": contained.unwrap_or_default(),
            "noHeader": noHeader.unwrap_or_default(),
            "handleDocumentKeyDown": handleDocumentKeyDown.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-dropdown`
#[component]
pub fn SlDropdown(
    dependencies: Option<String>,
    popup: Option<String>,
    trigger: Option<String>,
    panel: Option<String>,
    localize: Option<String>,
    closeWatcher: Option<String>,
    open: Option<bool>,
    placement: Option<String>,
    disabled: Option<bool>,
    stayOpenOnSelect: Option<bool>,
    containingElement: Option<String>,
    distance: Option<f64>,
    skidding: Option<f64>,
    hoist: Option<bool>,
    sync: Option<String>,
    handleKeyDown: Option<String>,
    handleDocumentKeyDown: Option<String>,
    handleDocumentMouseDown: Option<String>,
    handlePanelSelect: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-dropdown {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "popup": popup.unwrap_or_default(),
            "trigger": trigger.unwrap_or_default(),
            "panel": panel.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "closeWatcher": closeWatcher.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "placement": placement.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "stayOpenOnSelect": stayOpenOnSelect.unwrap_or_default(),
            "containingElement": containingElement.unwrap_or_default(),
            "distance": distance.unwrap_or_default(),
            "skidding": skidding.unwrap_or_default(),
            "hoist": hoist.unwrap_or_default(),
            "sync": sync.unwrap_or_default(),
            "handleKeyDown": handleKeyDown.unwrap_or_default(),
            "handleDocumentKeyDown": handleDocumentKeyDown.unwrap_or_default(),
            "handleDocumentMouseDown": handleDocumentMouseDown.unwrap_or_default(),
            "handlePanelSelect": handlePanelSelect.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-format-bytes`
#[component]
pub fn SlFormatBytes(
    localize: Option<String>,
    value: Option<f64>,
    unit: Option<String>,
    display: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-format-bytes {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "unit": unit.unwrap_or_default(),
            "display": display.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-format-date`
#[component]
pub fn SlFormatDate(
    localize: Option<String>,
    date: Option<String>,
    weekday: Option<String>,
    era: Option<String>,
    year: Option<String>,
    month: Option<String>,
    day: Option<String>,
    hour: Option<String>,
    minute: Option<String>,
    second: Option<String>,
    timeZoneName: Option<String>,
    timeZone: Option<String>,
    hourFormat: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-format-date {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            "date": date.unwrap_or_default(),
            "weekday": weekday.unwrap_or_default(),
            "era": era.unwrap_or_default(),
            "year": year.unwrap_or_default(),
            "month": month.unwrap_or_default(),
            "day": day.unwrap_or_default(),
            "hour": hour.unwrap_or_default(),
            "minute": minute.unwrap_or_default(),
            "second": second.unwrap_or_default(),
            "timeZoneName": timeZoneName.unwrap_or_default(),
            "timeZone": timeZone.unwrap_or_default(),
            "hourFormat": hourFormat.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-format-number`
#[component]
pub fn SlFormatNumber(
    localize: Option<String>,
    value: Option<f64>,
    r#type: Option<String>,
    noGrouping: Option<bool>,
    currency: Option<String>,
    currencyDisplay: Option<String>,
    minimumIntegerDigits: Option<f64>,
    minimumFractionDigits: Option<f64>,
    maximumFractionDigits: Option<f64>,
    minimumSignificantDigits: Option<f64>,
    maximumSignificantDigits: Option<f64>,
    children: Element,
) -> Element {
    rsx! {
        sl-format-number {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "type": r#type.unwrap_or_default(),
            "noGrouping": noGrouping.unwrap_or_default(),
            "currency": currency.unwrap_or_default(),
            "currencyDisplay": currencyDisplay.unwrap_or_default(),
            "minimumIntegerDigits": minimumIntegerDigits.unwrap_or_default(),
            "minimumFractionDigits": minimumFractionDigits.unwrap_or_default(),
            "maximumFractionDigits": maximumFractionDigits.unwrap_or_default(),
            "minimumSignificantDigits": minimumSignificantDigits.unwrap_or_default(),
            "maximumSignificantDigits": maximumSignificantDigits.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-icon`
#[component]
pub fn SlIcon(
    initialRender: Option<bool>,
    svg: Option<String>,
    name: Option<String>,
    src: Option<String>,
    label: Option<String>,
    library: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-icon {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "initialRender": initialRender.unwrap_or_default(),
            "svg": svg.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "src": src.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "library": library.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-include`
#[component]
pub fn SlInclude(
    src: Option<String>,
    mode: Option<String>,
    allowScripts: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-include {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "src": src.unwrap_or_default(),
            "mode": mode.unwrap_or_default(),
            "allowScripts": allowScripts.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-icon-button`
#[component]
pub fn SlIconButton(
    dependencies: Option<String>,
    button: Option<String>,
    hasFocus: Option<bool>,
    name: Option<String>,
    library: Option<String>,
    src: Option<String>,
    href: Option<String>,
    target: Option<String>,
    download: Option<String>,
    label: Option<String>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-icon-button {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "button": button.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "library": library.unwrap_or_default(),
            "src": src.unwrap_or_default(),
            "href": href.unwrap_or_default(),
            "target": target.unwrap_or_default(),
            "download": download.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            {children}
        }
    }
}
