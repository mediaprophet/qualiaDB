//! Misc components: Textarea, Tag, Tooltip, Tree, TreeItem, VisuallyHidden

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-textarea`
#[component]
pub fn SlTextarea(
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    resizeObserver: Option<String>,
    input: Option<String>,
    sizeAdjuster: Option<String>,
    hasFocus: Option<bool>,
    title: Option<String>,
    name: Option<String>,
    value: Option<String>,
    size: Option<String>,
    filled: Option<bool>,
    label: Option<String>,
    helpText: Option<String>,
    placeholder: Option<String>,
    rows: Option<f64>,
    resize: Option<String>,
    disabled: Option<bool>,
    readonly: Option<bool>,
    form: Option<String>,
    required: Option<bool>,
    minlength: Option<f64>,
    maxlength: Option<f64>,
    autocapitalize: Option<String>,
    autocorrect: Option<String>,
    autocomplete: Option<String>,
    autofocus: Option<bool>,
    enterkeyhint: Option<String>,
    spellcheck: Option<bool>,
    inputmode: Option<String>,
    defaultValue: Option<String>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-textarea {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "resizeObserver": resizeObserver.unwrap_or_default(),
            "input": input.unwrap_or_default(),
            "sizeAdjuster": sizeAdjuster.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "title": title.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "filled": filled.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "helpText": helpText.unwrap_or_default(),
            "placeholder": placeholder.unwrap_or_default(),
            "rows": rows.unwrap_or_default(),
            "resize": resize.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "readonly": readonly.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "required": required.unwrap_or_default(),
            "minlength": minlength.unwrap_or_default(),
            "maxlength": maxlength.unwrap_or_default(),
            "autocapitalize": autocapitalize.unwrap_or_default(),
            "autocorrect": autocorrect.unwrap_or_default(),
            "autocomplete": autocomplete.unwrap_or_default(),
            "autofocus": autofocus.unwrap_or_default(),
            "enterkeyhint": enterkeyhint.unwrap_or_default(),
            "spellcheck": spellcheck.unwrap_or_default(),
            "inputmode": inputmode.unwrap_or_default(),
            "defaultValue": defaultValue.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tag`
#[component]
pub fn SlTag(
    dependencies: Option<String>,
    localize: Option<String>,
    variant: Option<String>,
    size: Option<String>,
    pill: Option<bool>,
    removable: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-tag {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "variant": variant.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "pill": pill.unwrap_or_default(),
            "removable": removable.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tooltip`
#[component]
pub fn SlTooltip(
    dependencies: Option<String>,
    hoverTimeout: Option<f64>,
    localize: Option<String>,
    closeWatcher: Option<String>,
    defaultSlot: Option<String>,
    body: Option<String>,
    popup: Option<String>,
    content: Option<String>,
    placement: Option<String>,
    disabled: Option<bool>,
    distance: Option<f64>,
    open: Option<bool>,
    skidding: Option<f64>,
    trigger: Option<String>,
    hoist: Option<bool>,
    handleBlur: Option<String>,
    handleClick: Option<String>,
    handleFocus: Option<String>,
    handleDocumentKeyDown: Option<String>,
    handleMouseOver: Option<String>,
    handleMouseOut: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-tooltip {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "hoverTimeout": hoverTimeout.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "closeWatcher": closeWatcher.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "body": body.unwrap_or_default(),
            "popup": popup.unwrap_or_default(),
            "content": content.unwrap_or_default(),
            "placement": placement.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "distance": distance.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "skidding": skidding.unwrap_or_default(),
            "trigger": trigger.unwrap_or_default(),
            "hoist": hoist.unwrap_or_default(),
            "handleBlur": handleBlur.unwrap_or_default(),
            "handleClick": handleClick.unwrap_or_default(),
            "handleFocus": handleFocus.unwrap_or_default(),
            "handleDocumentKeyDown": handleDocumentKeyDown.unwrap_or_default(),
            "handleMouseOver": handleMouseOver.unwrap_or_default(),
            "handleMouseOut": handleMouseOut.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tree`
#[component]
pub fn SlTree(
    defaultSlot: Option<String>,
    expandedIconSlot: Option<String>,
    collapsedIconSlot: Option<String>,
    selection: Option<String>,
    lastFocusedItem: Option<String>,
    mutationObserver: Option<String>,
    clickTarget: Option<String>,
    localize: Option<String>,
    initTreeItem: Option<String>,
    handleTreeChanged: Option<String>,
    handleFocusOut: Option<String>,
    handleFocusIn: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-tree {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "expandedIconSlot": expandedIconSlot.unwrap_or_default(),
            "collapsedIconSlot": collapsedIconSlot.unwrap_or_default(),
            "selection": selection.unwrap_or_default(),
            "lastFocusedItem": lastFocusedItem.unwrap_or_default(),
            "mutationObserver": mutationObserver.unwrap_or_default(),
            "clickTarget": clickTarget.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "initTreeItem": initTreeItem.unwrap_or_default(),
            "handleTreeChanged": handleTreeChanged.unwrap_or_default(),
            "handleFocusOut": handleFocusOut.unwrap_or_default(),
            "handleFocusIn": handleFocusIn.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-tree-item`
#[component]
pub fn SlTreeItem(
    dependencies: Option<String>,
    localize: Option<String>,
    indeterminate: Option<bool>,
    isLeaf: Option<bool>,
    loading: Option<bool>,
    selectable: Option<bool>,
    expanded: Option<bool>,
    selected: Option<bool>,
    disabled: Option<bool>,
    lazy: Option<bool>,
    defaultSlot: Option<String>,
    childrenSlot: Option<String>,
    itemElement: Option<String>,
    childrenContainer: Option<String>,
    expandButtonSlot: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-tree-item {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "indeterminate": indeterminate.unwrap_or_default(),
            "isLeaf": isLeaf.unwrap_or_default(),
            "loading": loading.unwrap_or_default(),
            "selectable": selectable.unwrap_or_default(),
            "expanded": expanded.unwrap_or_default(),
            "selected": selected.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "lazy": lazy.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "childrenSlot": childrenSlot.unwrap_or_default(),
            "itemElement": itemElement.unwrap_or_default(),
            "childrenContainer": childrenContainer.unwrap_or_default(),
            "expandButtonSlot": expandButtonSlot.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-visually-hidden`
#[component]
pub fn SlVisuallyHidden(children: Element) -> Element {
    rsx! {
        sl-visually-hidden {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            {children}
        }
    }
}
