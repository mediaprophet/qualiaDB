//! Layout components: Skeleton, Select, Spinner, Switch, SplitPanel, TabGroup, Tab, TabPanel

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-skeleton`
#[component]
pub fn SlSkeleton(effect: Option<String>, children: Element) -> Element {
    rsx! {
        sl-skeleton {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "effect": effect.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-select`
#[component]
pub fn SlSelect(
    dependencies: Option<String>,
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    typeToSelectString: Option<String>,
    typeToSelectTimeout: Option<f64>,
    closeWatcher: Option<String>,
    popup: Option<String>,
    combobox: Option<String>,
    displayInput: Option<String>,
    valueInput: Option<String>,
    listbox: Option<String>,
    hasFocus: Option<bool>,
    displayLabel: Option<String>,
    currentOption: Option<String>,
    selectedOptions: Option<String>,
    valueHasChanged: Option<bool>,
    name: Option<String>,
    value: Option<String>,
    defaultValue: Option<String>,
    size: Option<String>,
    placeholder: Option<String>,
    multiple: Option<bool>,
    maxOptionsVisible: Option<f64>,
    disabled: Option<bool>,
    clearable: Option<bool>,
    open: Option<bool>,
    hoist: Option<bool>,
    filled: Option<bool>,
    pill: Option<bool>,
    label: Option<String>,
    placement: Option<String>,
    helpText: Option<String>,
    form: Option<String>,
    required: Option<bool>,
    getTag: Option<String>,
    validity: Option<String>,
    validationMessage: Option<String>,
    handleDocumentFocusIn: Option<String>,
    handleDocumentKeyDown: Option<String>,
    handleDocumentMouseDown: Option<String>,
    tags: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-select {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "typeToSelectString": typeToSelectString.unwrap_or_default(),
            "typeToSelectTimeout": typeToSelectTimeout.unwrap_or_default(),
            "closeWatcher": closeWatcher.unwrap_or_default(),
            "popup": popup.unwrap_or_default(),
            "combobox": combobox.unwrap_or_default(),
            "displayInput": displayInput.unwrap_or_default(),
            "valueInput": valueInput.unwrap_or_default(),
            "listbox": listbox.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "displayLabel": displayLabel.unwrap_or_default(),
            "currentOption": currentOption.unwrap_or_default(),
            "selectedOptions": selectedOptions.unwrap_or_default(),
            "valueHasChanged": valueHasChanged.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "defaultValue": defaultValue.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "placeholder": placeholder.unwrap_or_default(),
            "multiple": multiple.unwrap_or_default(),
            "maxOptionsVisible": maxOptionsVisible.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "clearable": clearable.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "hoist": hoist.unwrap_or_default(),
            "filled": filled.unwrap_or_default(),
            "pill": pill.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "placement": placement.unwrap_or_default(),
            "helpText": helpText.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "required": required.unwrap_or_default(),
            "getTag": getTag.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            "handleDocumentFocusIn": handleDocumentFocusIn.unwrap_or_default(),
            "handleDocumentKeyDown": handleDocumentKeyDown.unwrap_or_default(),
            "handleDocumentMouseDown": handleDocumentMouseDown.unwrap_or_default(),
            "tags": tags.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-spinner`
#[component]
pub fn SlSpinner(localize: Option<String>, children: Element) -> Element {
    rsx! {
        sl-spinner {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "localize": localize.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-switch`
#[component]
pub fn SlSwitch(
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
    defaultChecked: Option<bool>,
    form: Option<String>,
    required: Option<bool>,
    helpText: Option<String>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-switch {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
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

/// Dioxus wrapper for `sl-split-panel`
#[component]
pub fn SlSplitPanel(
    cachedPositionInPixels: Option<f64>,
    isCollapsed: Option<bool>,
    localize: Option<String>,
    positionBeforeCollapsing: Option<f64>,
    resizeObserver: Option<String>,
    size: Option<f64>,
    divider: Option<String>,
    position: Option<f64>,
    positionInPixels: Option<f64>,
    vertical: Option<bool>,
    disabled: Option<bool>,
    primary: Option<String>,
    snapValue: Option<String>,
    snapFunction: Option<String>,
    snap: Option<String>,
    snapThreshold: Option<f64>,
    children: Element,
) -> Element {
    rsx! {
        sl-split-panel {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "cachedPositionInPixels": cachedPositionInPixels.unwrap_or_default(),
            "isCollapsed": isCollapsed.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "positionBeforeCollapsing": positionBeforeCollapsing.unwrap_or_default(),
            "resizeObserver": resizeObserver.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "divider": divider.unwrap_or_default(),
            "position": position.unwrap_or_default(),
            "positionInPixels": positionInPixels.unwrap_or_default(),
            "vertical": vertical.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "primary": primary.unwrap_or_default(),
            "snapValue": snapValue.unwrap_or_default(),
            "snapFunction": snapFunction.unwrap_or_default(),
            "snap": snap.unwrap_or_default(),
            "snapThreshold": snapThreshold.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tab-group`
#[component]
pub fn SlTabGroup(
    dependencies: Option<String>,
    activeTab: Option<String>,
    mutationObserver: Option<String>,
    resizeObserver: Option<String>,
    tabs: Option<String>,
    focusableTabs: Option<String>,
    panels: Option<String>,
    localize: Option<String>,
    tabGroup: Option<String>,
    body: Option<String>,
    nav: Option<String>,
    indicator: Option<String>,
    hasScrollControls: Option<bool>,
    shouldHideScrollStartButton: Option<bool>,
    shouldHideScrollEndButton: Option<bool>,
    placement: Option<String>,
    activation: Option<String>,
    noScrollControls: Option<bool>,
    fixedScrollControls: Option<bool>,
    scrollOffset: Option<f64>,
    children: Element,
) -> Element {
    rsx! {
        sl-tab-group {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "activeTab": activeTab.unwrap_or_default(),
            "mutationObserver": mutationObserver.unwrap_or_default(),
            "resizeObserver": resizeObserver.unwrap_or_default(),
            "tabs": tabs.unwrap_or_default(),
            "focusableTabs": focusableTabs.unwrap_or_default(),
            "panels": panels.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "tabGroup": tabGroup.unwrap_or_default(),
            "body": body.unwrap_or_default(),
            "nav": nav.unwrap_or_default(),
            "indicator": indicator.unwrap_or_default(),
            "hasScrollControls": hasScrollControls.unwrap_or_default(),
            "shouldHideScrollStartButton": shouldHideScrollStartButton.unwrap_or_default(),
            "shouldHideScrollEndButton": shouldHideScrollEndButton.unwrap_or_default(),
            "placement": placement.unwrap_or_default(),
            "activation": activation.unwrap_or_default(),
            "noScrollControls": noScrollControls.unwrap_or_default(),
            "fixedScrollControls": fixedScrollControls.unwrap_or_default(),
            "scrollOffset": scrollOffset.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tab`
#[component]
pub fn SlTab(
    dependencies: Option<String>,
    localize: Option<String>,
    attrId: Option<f64>,
    componentId: Option<String>,
    tab: Option<String>,
    panel: Option<String>,
    active: Option<bool>,
    closable: Option<bool>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-tab {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "attrId": attrId.unwrap_or_default(),
            "componentId": componentId.unwrap_or_default(),
            "tab": tab.unwrap_or_default(),
            "panel": panel.unwrap_or_default(),
            "active": active.unwrap_or_default(),
            "closable": closable.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tab-panel`
#[component]
pub fn SlTabPanel(
    attrId: Option<f64>,
    componentId: Option<String>,
    name: Option<String>,
    active: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-tab-panel {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "attrId": attrId.unwrap_or_default(),
            "componentId": componentId.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "active": active.unwrap_or_default(),
            {children}
        }
    }
}
