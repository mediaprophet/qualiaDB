//! Basic components: Alert, AnimatedImage, Animation, Avatar, Badge, Breadcrumb, Button, BreadcrumbItem, ButtonGroup, Card

#![allow(non_snake_case)]
use core::option::Option;
use dioxus::prelude::*;

/// Dioxus wrapper for `sl-alert`
#[component]
pub fn SlAlert(
    dependencies: Option<String>,
    autoHideTimeout: Option<f64>,
    remainingTimeInterval: Option<f64>,
    countdownAnimation: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    currentToastStack: Option<String>,
    toastStack: Option<String>,
    base: Option<String>,
    countdownElement: Option<String>,
    open: Option<bool>,
    closable: Option<bool>,
    variant: Option<String>,
    duration: Option<String>,
    countdown: Option<String>,
    remainingTime: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-alert {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "autoHideTimeout": autoHideTimeout.unwrap_or_default(),
            "remainingTimeInterval": remainingTimeInterval.unwrap_or_default(),
            "countdownAnimation": countdownAnimation.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "currentToastStack": currentToastStack.unwrap_or_default(),
            "toastStack": toastStack.unwrap_or_default(),
            "base": base.unwrap_or_default(),
            "countdownElement": countdownElement.unwrap_or_default(),
            "open": open.unwrap_or_default(),
            "closable": closable.unwrap_or_default(),
            "variant": variant.unwrap_or_default(),
            "duration": duration.unwrap_or_default(),
            "countdown": countdown.unwrap_or_default(),
            "remainingTime": remainingTime.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-animated-image`
#[component]
pub fn SlAnimatedImage(
    dependencies: Option<String>,
    animatedImage: Option<String>,
    frozenFrame: Option<String>,
    isLoaded: Option<bool>,
    src: Option<String>,
    alt: Option<String>,
    play: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-animated-image {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "animatedImage": animatedImage.unwrap_or_default(),
            "frozenFrame": frozenFrame.unwrap_or_default(),
            "isLoaded": isLoaded.unwrap_or_default(),
            "src": src.unwrap_or_default(),
            "alt": alt.unwrap_or_default(),
            "play": play.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-animation`
#[component]
pub fn SlAnimation(
    animation: Option<String>,
    hasStarted: Option<bool>,
    defaultSlot: Option<String>,
    name: Option<String>,
    play: Option<bool>,
    delay: Option<f64>,
    direction: Option<String>,
    duration: Option<f64>,
    easing: Option<String>,
    endDelay: Option<f64>,
    fill: Option<String>,
    iterations: Option<String>,
    iterationStart: Option<f64>,
    keyframes: Option<String>,
    playbackRate: Option<f64>,
    currentTime: Option<String>,
    handleAnimationFinish: Option<String>,
    handleAnimationCancel: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-animation {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "animation": animation.unwrap_or_default(),
            "hasStarted": hasStarted.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "play": play.unwrap_or_default(),
            "delay": delay.unwrap_or_default(),
            "direction": direction.unwrap_or_default(),
            "duration": duration.unwrap_or_default(),
            "easing": easing.unwrap_or_default(),
            "endDelay": endDelay.unwrap_or_default(),
            "fill": fill.unwrap_or_default(),
            "iterations": iterations.unwrap_or_default(),
            "iterationStart": iterationStart.unwrap_or_default(),
            "keyframes": keyframes.unwrap_or_default(),
            "playbackRate": playbackRate.unwrap_or_default(),
            "currentTime": currentTime.unwrap_or_default(),
            "handleAnimationFinish": handleAnimationFinish.unwrap_or_default(),
            "handleAnimationCancel": handleAnimationCancel.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-avatar`
#[component]
pub fn SlAvatar(
    dependencies: Option<String>,
    hasError: Option<bool>,
    image: Option<String>,
    label: Option<String>,
    initials: Option<String>,
    loading: Option<String>,
    shape: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-avatar {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "hasError": hasError.unwrap_or_default(),
            "image": image.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            "initials": initials.unwrap_or_default(),
            "loading": loading.unwrap_or_default(),
            "shape": shape.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-badge`
#[component]
pub fn SlBadge(
    variant: Option<String>,
    pill: Option<bool>,
    pulse: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        sl-badge {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "variant": variant.unwrap_or_default(),
            "pill": pill.unwrap_or_default(),
            "pulse": pulse.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-breadcrumb`
#[component]
pub fn SlBreadcrumb(
    dependencies: Option<String>,
    localize: Option<String>,
    separatorDir: Option<String>,
    defaultSlot: Option<String>,
    separatorSlot: Option<String>,
    label: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-breadcrumb {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "separatorDir": separatorDir.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "separatorSlot": separatorSlot.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-button`
#[component]
pub fn SlButton(
    dependencies: Option<String>,
    formControlController: Option<String>,
    hasSlotController: Option<String>,
    localize: Option<String>,
    button: Option<String>,
    hasFocus: Option<bool>,
    invalid: Option<bool>,
    title: Option<String>,
    variant: Option<String>,
    size: Option<String>,
    caret: Option<bool>,
    disabled: Option<bool>,
    loading: Option<bool>,
    outline: Option<bool>,
    pill: Option<bool>,
    circle: Option<bool>,
    r#type: Option<String>,
    name: Option<String>,
    value: Option<String>,
    href: Option<String>,
    target: Option<String>,
    rel: Option<String>,
    download: Option<String>,
    form: Option<String>,
    formAction: Option<String>,
    formEnctype: Option<String>,
    formMethod: Option<String>,
    formNoValidate: Option<bool>,
    formTarget: Option<String>,
    validity: Option<String>,
    validationMessage: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-button {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "dependencies": dependencies.unwrap_or_default(),
            "formControlController": formControlController.unwrap_or_default(),
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "localize": localize.unwrap_or_default(),
            "button": button.unwrap_or_default(),
            "hasFocus": hasFocus.unwrap_or_default(),
            "invalid": invalid.unwrap_or_default(),
            "title": title.unwrap_or_default(),
            "variant": variant.unwrap_or_default(),
            "size": size.unwrap_or_default(),
            "caret": caret.unwrap_or_default(),
            "disabled": disabled.unwrap_or_default(),
            "loading": loading.unwrap_or_default(),
            "outline": outline.unwrap_or_default(),
            "pill": pill.unwrap_or_default(),
            "circle": circle.unwrap_or_default(),
            "type": r#type.unwrap_or_default(),
            "name": name.unwrap_or_default(),
            "value": value.unwrap_or_default(),
            "href": href.unwrap_or_default(),
            "target": target.unwrap_or_default(),
            "rel": rel.unwrap_or_default(),
            "download": download.unwrap_or_default(),
            "form": form.unwrap_or_default(),
            "formAction": formAction.unwrap_or_default(),
            "formEnctype": formEnctype.unwrap_or_default(),
            "formMethod": formMethod.unwrap_or_default(),
            "formNoValidate": formNoValidate.unwrap_or_default(),
            "formTarget": formTarget.unwrap_or_default(),
            "validity": validity.unwrap_or_default(),
            "validationMessage": validationMessage.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-breadcrumb-item`
#[component]
pub fn SlBreadcrumbItem(
    hasSlotController: Option<String>,
    defaultSlot: Option<String>,
    renderType: Option<String>,
    href: Option<String>,
    target: Option<String>,
    rel: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-breadcrumb-item {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "hasSlotController": hasSlotController.unwrap_or_default(),
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "renderType": renderType.unwrap_or_default(),
            "href": href.unwrap_or_default(),
            "target": target.unwrap_or_default(),
            "rel": rel.unwrap_or_default(),
            {children}
        }
    }
}

/// Dioxus wrapper for `sl-button-group`
#[component]
pub fn SlButtonGroup(
    defaultSlot: Option<String>,
    disableRole: Option<bool>,
    label: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        sl-button-group {
            style: "--sl-color-primary-500: var(--qualia-primary); --sl-color-primary-600: var(--qualia-primary-hover); --sl-color-success-500: var(--qualia-success); --sl-color-danger-500: var(--qualia-danger); --sl-color-neutral-500: var(--qualia-neutral); --sl-color-neutral-0: var(--qualia-bg);",
            "defaultSlot": defaultSlot.unwrap_or_default(),
            "disableRole": disableRole.unwrap_or_default(),
            "label": label.unwrap_or_default(),
            {children}
        }
    }
}
