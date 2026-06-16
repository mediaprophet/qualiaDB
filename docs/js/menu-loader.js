// QualiaDB site navigation — loads menu.json, renders desktop dropdowns + mobile drawer.
// Resolves paths from any docs subdirectory via the script tag location.

function docsRootFromScript() {
    const script = document.querySelector('script[src*="menu-loader.js"]');
    if (!script) return '';
    return script.getAttribute('src').replace(/js\/menu-loader\.js.*$/, '') || '';
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
        // Directory entries (trailing slash)
        if (href.endsWith('/') && window.location.pathname.startsWith(targetUrl.pathname)) return true;
    } catch (_) { /* ignore */ }
    const leaf = href.split('/').pop() || href;
    return window.location.pathname.endsWith('/' + leaf) || window.location.pathname.endsWith(leaf);
}

async function loadMenu() {
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

    navHost.className = 'relative flex items-center text-sm flex-1 justify-end md:justify-start md:flex-none md:gap-2';

    const grouped = {};
    for (const item of menu.navigation) {
        const group = item.group || 'More';
        (grouped[group] ||= []).push(item);
    }

    let html = `
        <button type="button" id="mobile-menu-btn" aria-label="Open menu"
            class="md:hidden text-slate-300 hover:text-white p-2 rounded-lg border border-white/10">
            <i class="fa-solid fa-bars text-lg"></i>
        </button>
        <div id="desktop-nav" class="hidden md:flex items-center gap-4 lg:gap-6 text-sm">
    `;

    for (const [groupName, items] of Object.entries(grouped)) {
        if (items.length === 1) {
            const item = items[0];
            const active = pageMatchesHref(item.href) ? 'nav-active font-medium text-white' : '';
            const tone = item.highlight ? 'text-cyan-400 hover:text-cyan-300' : 'text-slate-300 hover:text-white';
            const icon = item.icon ? `<i class="${normalizeIcon(item.icon)} mr-1"></i>` : '';
            html += `<a href="${resolveHref(item.href)}" class="${tone} ${active} transition-colors inline-flex items-center whitespace-nowrap">${icon}${item.label}</a>`;
        } else {
            html += `
                <div class="relative group">
                    <button type="button" class="text-slate-300 hover:text-white transition-colors inline-flex items-center gap-1 whitespace-nowrap">
                        ${groupName}
                        <i class="fa-solid fa-chevron-down text-[10px]"></i>
                    </button>
                    <div class="absolute left-0 mt-2 min-w-[12rem] bg-slate-900/98 border border-slate-700 rounded-xl shadow-xl
                                opacity-0 invisible group-hover:opacity-100 group-hover:visible group-focus-within:opacity-100
                                group-focus-within:visible transition-all duration-150 z-[60]">
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

    // Full-width mobile drawer attached to nav bar
    let mobileNav = document.getElementById('mobile-nav');
    if (!mobileNav) {
        mobileNav = document.createElement('div');
        mobileNav.id = 'mobile-nav';
        const anchor = navBar?.querySelector(':scope > div') || navBar;
        (anchor || document.body).appendChild(mobileNav);
    }

    mobileNav.className = 'hidden md:hidden absolute left-0 right-0 top-full z-[60] bg-slate-950/98 border-t border-slate-700 shadow-2xl max-h-[70vh] overflow-y-auto';
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