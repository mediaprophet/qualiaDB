// QualiaDB site navigation — loads menu.json, renders desktop dropdowns + mobile drawer.
// Resolves paths from any docs subdirectory via the script tag location.

function docsRootFromScript() {
    const script = document.querySelector('script[src*="menu-loader.js"]');
    let root = '';
    if (script) {
        const src = script.getAttribute('src') || '';
        if (src.startsWith('http') || src.startsWith('/')) {
            root = src.replace(/js\/menu-loader\.js.*$/, '');
        } else {
            root = src.replace(/js\/menu-loader\.js.*$/, '') || '';
        }
    }
    // GitHub Pages serves under /qualiaDB/ — infer when script path is page-relative.
    const pagesBase = window.location.pathname.match(/^(.*\/qualiaDB\/)/);
    if (pagesBase) {
        const base = pagesBase[1];
        if (!root || (!root.startsWith('/') && !root.startsWith('http'))) {
            return base;
        }
    }
    return root;
}

function ensureSiteNavCss() {
    const root = docsRootFromScript();
    if (!root || document.querySelector('link[data-site-nav]')) return;
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = root + 'css/site-nav.css';
    link.setAttribute('data-site-nav', '1');
    document.head.appendChild(link);
}

function resolveHref(href) {
    if (!href || href.startsWith('http') || href.startsWith('#')) return href;
    const root = docsRootFromScript();
    return root + href;
}

function normalizeIcon(icon) {
    if (!icon) return '';
    if (icon.includes('fa-')) {
        if (icon.startsWith('fa-solid') || icon.startsWith('fa-regular') || icon.startsWith('fab ') || icon.startsWith('fas ')) {
            return icon;
        }
        return `fa-solid ${icon}`;
    }
    return icon;
}

function pageMatchesHref(href) {
    const target = resolveHref(href);
    try {
        const targetUrl = new URL(target, window.location.href);
        if (targetUrl.pathname === window.location.pathname) return true;
        if (href.endsWith('/') && window.location.pathname.startsWith(targetUrl.pathname)) return true;
    } catch (_) { /* ignore */ }
    const leaf = (href.split('/').pop() || href).split('?')[0];
    const path = window.location.pathname;
    return path.endsWith('/' + leaf) || path.endsWith(leaf);
}

async function loadMenu() {
    ensureSiteNavCss();
    const root = docsRootFromScript();
    try {
        const response = await fetch(root + 'menu.json');
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        renderMenu(await response.json());
    } catch (error) {
        console.error('Failed to load menu:', error);
        renderFallbackMenu();
    }
}

function renderMenu(menu) {
    const navHost = document.getElementById('dynamic-nav') || findLegacyNavHost();
    if (!navHost) {
        console.warn('menu-loader: no #dynamic-nav element');
        return;
    }

    const navBar = navHost.closest('nav');
    if (navBar) navBar.classList.add('relative', 'z-50');

    navHost.className = 'relative flex items-center gap-2 text-sm min-w-0 flex-1 justify-center lg:justify-start px-2 max-w-full';

    const grouped = {};
    for (const item of menu.navigation) {
        const group = item.group || 'More';
        (grouped[group] ||= []).push(item);
    }

    let html = `
        <button type="button" id="mobile-menu-btn" aria-label="Open site menu"
            class="lg:hidden shrink-0 text-slate-300 hover:text-white p-2 rounded-lg border border-white/10">
            <i class="fa-solid fa-bars text-lg"></i>
        </button>
        <div id="desktop-nav" class="hidden lg:flex flex-nowrap items-center gap-2 xl:gap-4 text-sm max-w-full">
    `;

    for (const [groupName, items] of Object.entries(grouped)) {
        if (items.length === 1) {
            const item = items[0];
            const active = pageMatchesHref(item.href) ? 'nav-active font-medium text-white' : '';
            const tone = item.highlight ? 'text-cyan-400 hover:text-cyan-300' : 'text-slate-300 hover:text-white';
            const icon = item.icon ? `<i class="${normalizeIcon(item.icon)} mr-1"></i>` : '';
            html += `<a href="${resolveHref(item.href)}" class="${tone} ${active} transition-colors inline-flex items-center whitespace-nowrap px-1">${icon}${item.label}</a>`;
        } else {
            html += `
                <div class="nav-group relative group shrink-0">
                    <button type="button" class="text-slate-300 hover:text-white transition-colors inline-flex items-center gap-1 whitespace-nowrap px-1">
                        ${groupName}
                        <i class="fa-solid fa-chevron-down text-[10px]"></i>
                    </button>
                    <div class="nav-dropdown">
                        <div class="py-2">
                            ${items.map(item => {
                                const active = pageMatchesHref(item.href) ? 'nav-active font-medium text-white' : '';
                                const tone = item.highlight ? 'text-cyan-400 hover:text-cyan-300' : 'text-slate-300 hover:text-white';
                                const icon = item.icon ? `<i class="${normalizeIcon(item.icon)} mr-2 w-4"></i>` : '';
                                return `<a href="${resolveHref(item.href)}" class="${tone} ${active} block px-4 py-2 transition-colors flex items-center">${icon}${item.label}</a>`;
                            }).join('')}
                        </div>
                    </div>
                </div>`;
        }
    }

    html += `</div>`;
    navHost.innerHTML = html;

    let mobileNav = document.getElementById('mobile-nav');
    if (!mobileNav) {
        mobileNav = document.createElement('div');
        mobileNav.id = 'mobile-nav';
        (navBar || document.body).appendChild(mobileNav);
    }

    mobileNav.className = 'hidden lg:hidden absolute left-0 right-0 top-full z-[70] bg-slate-950/98 border-t border-slate-700 shadow-2xl max-h-[75vh] overflow-y-auto';
    mobileNav.innerHTML = `<div class="py-3 px-4 space-y-1 max-w-screen-2xl mx-auto">${
        Object.entries(grouped).map(([groupName, items]) => {
            if (items.length === 1) {
                const item = items[0];
                const active = pageMatchesHref(item.href) ? 'nav-active font-medium text-white' : '';
                const tone = item.highlight ? 'text-cyan-400' : 'text-slate-200 hover:text-white';
                const icon = item.icon ? `<i class="${normalizeIcon(item.icon)} mr-2"></i>` : '';
                return `<a href="${resolveHref(item.href)}" class="${tone} ${active} block px-3 py-2.5 rounded-lg">${icon}${item.label}</a>`;
            }
            return `
                <div class="mobile-dropdown border-b border-slate-800/80 pb-2 mb-2 last:border-0">
                    <button type="button" class="mobile-dropdown-btn w-full text-left px-3 py-2 text-slate-400 text-xs uppercase tracking-wider flex items-center justify-between">
                        <span>${groupName}</span>
                        <i class="fa-solid fa-chevron-down text-[10px] transition-transform"></i>
                    </button>
                    <div class="mobile-dropdown-content hidden pl-1 mt-1 space-y-0.5">
                        ${items.map(item => {
                            const active = pageMatchesHref(item.href) ? 'nav-active font-medium text-white' : '';
                            const tone = item.highlight ? 'text-cyan-400' : 'text-slate-200 hover:text-white';
                            const icon = item.icon ? `<i class="${normalizeIcon(item.icon)} mr-2"></i>` : '';
                            return `<a href="${resolveHref(item.href)}" class="${tone} ${active} block px-3 py-2.5 rounded-lg">${icon}${item.label}</a>`;
                        }).join('')}
                    </div>
                </div>`;
        }).join('')
    }</div>`;

    wireMobileMenu();
    wireDesktopDropdowns();
    updateBrand(menu);
    updateFooter(menu);
}

function findLegacyNavHost() {
    const selectors = [
        'nav .hidden.md\\:flex',
        'nav .flex.items-center.gap-6.text-sm',
        'nav .flex.items-center.gap-x-1.text-sm',
    ];
    for (const sel of selectors) {
        const el = document.querySelector(sel);
        if (el) {
            el.id = 'dynamic-nav';
            return el;
        }
    }
    return null;
}

function wireDesktopDropdowns() {
    const desktop = document.getElementById('desktop-nav');
    if (!desktop) return;

    desktop.querySelectorAll('.nav-group > button').forEach((btn) => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const group = btn.closest('.nav-group');
            if (!group) return;
            const open = group.classList.contains('nav-open');
            desktop.querySelectorAll('.nav-group').forEach((g) => g.classList.remove('nav-open'));
            if (!open) group.classList.add('nav-open');
        });
    });

    document.addEventListener('click', () => {
        desktop.querySelectorAll('.nav-group').forEach((g) => g.classList.remove('nav-open'));
    });
}

function wireMobileMenu() {
    const btn = document.getElementById('mobile-menu-btn');
    const panel = document.getElementById('mobile-nav');
    if (!btn || !panel) return;

    btn.addEventListener('click', (e) => {
        e.stopPropagation();
        panel.classList.toggle('hidden');
        const open = !panel.classList.contains('hidden');
        btn.innerHTML = open
            ? '<i class="fa-solid fa-xmark text-lg"></i>'
            : '<i class="fa-solid fa-bars text-lg"></i>';
    });

    document.addEventListener('click', (e) => {
        if (!btn.contains(e.target) && !panel.contains(e.target)) {
            panel.classList.add('hidden');
            btn.innerHTML = '<i class="fa-solid fa-bars text-lg"></i>';
        }
    });

    panel.querySelectorAll('a').forEach((a) => {
        a.addEventListener('click', () => {
            panel.classList.add('hidden');
            btn.innerHTML = '<i class="fa-solid fa-bars text-lg"></i>';
        });
    });

    document.querySelectorAll('.mobile-dropdown-btn').forEach((dropBtn) => {
        dropBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            const content = dropBtn.nextElementSibling;
            const chevron = dropBtn.querySelector('.fa-chevron-down');
            content?.classList.toggle('hidden');
            chevron?.classList.toggle('rotate-180');
        });
    });
}

function updateBrand(menu) {
    const brandLink = document.querySelector('nav a[href*="index.html"]');
    if (!brandLink || !menu.brand) return;
    brandLink.href = resolveHref('index.html');
    brandLink.innerHTML = `
        <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-cyan-400 rounded-xl flex items-center justify-center shrink-0">
            <span class="text-black font-bold text-xl">${menu.brand.icon}</span>
        </div>
        <div class="hidden sm:block"><span class="font-semibold tracking-tight text-xl">${menu.brand.name.replace('DB', '')}</span><span class="text-blue-400 text-xl">DB</span></div>
    `;
}

function updateFooter(menu) {
    if (!menu.footer?.links) return;
    const footerLinks = document.querySelector('footer .flex');
    if (!footerLinks) return;
    footerLinks.innerHTML = menu.footer.links.map((link) => `
        <a href="${resolveHref(link.href)}" class="hover:text-white transition-colors flex items-center gap-2" target="${link.href.startsWith('http') ? '_blank' : '_self'}">
            <i class="${normalizeIcon(link.icon)}"></i><span>${link.label}</span>
        </a>
    `).join('');
}

function renderFallbackMenu() {
    renderMenu({
        brand: { name: 'QualiaDB', icon: 'Q' },
        navigation: [
            { label: 'Home', href: 'index.html', group: 'Core' },
            { label: 'Benchmark', href: 'benchmark.html', group: 'Benchmarks' },
            { label: 'Ontology', href: 'ontology.html', group: 'Data' },
            { label: 'API Docs', href: 'api.html', group: 'Core' },
            { label: 'Manuals', href: 'manuals/index.html', group: 'Core' },
        ],
        footer: { links: [] },
    });
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', loadMenu);
} else {
    loadMenu();
}