// Menu loader for consistent navigation across all QualiaDB documentation pages
// Loads menu.json and renders navigation dynamically

async function loadMenu() {
    try {
        const response = await fetch('menu.json');
        const menu = await response.json();
        renderMenu(menu);
    } catch (error) {
        console.error('Failed to load menu:', error);
        renderFallbackMenu();
    }
}

function renderMenu(menu) {
    // Try multiple selectors for different page layouts
    const navSelectors = [
        'nav .hidden.md\\:flex',
        'nav .flex.items-center.gap-6.text-sm',
        'nav .flex.items-center.gap-6'
    ];

    let nav = null;
    for (const selector of navSelectors) {
        nav = document.querySelector(selector);
        if (nav) break;
    }

    if (!nav) {
        console.warn('Could not find navigation element');
        return;
    }

    // Get current page for active state
    const currentPage = window.location.pathname.split('/').pop() || 'index.html';

    nav.innerHTML = menu.navigation.map(item => {
        const isActive = item.href === currentPage ? 'nav-active font-medium' : '';
        const highlight = item.highlight ? 'text-cyan-400' : 'hover:text-white';
        const icon = item.icon ? `<i class="${item.icon} mr-1"></i>` : '';
        return `<a href="${item.href}" class="${highlight} ${isActive} transition-colors flex items-center">${icon}${item.label}</a>`;
    }).join('');

    // Update brand
    const brandLink = document.querySelector('nav a[href="index.html"], nav a[href^="index.html"]');
    if (brandLink) {
        brandLink.innerHTML = `
            <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-cyan-400 rounded-xl flex items-center justify-center">
                <span class="text-black font-bold text-xl">${menu.brand.icon}</span>
            </div>
            <div><span class="font-semibold tracking-tight text-xl">${menu.brand.name}</span><span class="text-blue-400 text-xl">DB</span></div>
        `;
    }

    // Update footer links if footer exists
    const footerLinks = document.querySelector('footer .flex');
    if (footerLinks) {
        footerLinks.innerHTML = menu.footer.links.map(link => `
            <a href="${link.href}" class="hover:text-white transition-colors flex items-center gap-2">
                <i class="${link.icon}"></i><span>${link.label}</span>
            </a>
        `).join('');
    }

    // Update page title with version
    const versionDisplay = document.querySelector('[data-version]');
    if (versionDisplay) {
        versionDisplay.textContent = menu.version;
    }
}

function renderFallbackMenu() {
    // Fallback hardcoded menu if JSON fails to load
    const navSelectors = [
        'nav .hidden.md\\:flex',
        'nav .flex.items-center.gap-6.text-sm',
        'nav .flex.items-center.gap-6'
    ];

    let nav = null;
    for (const selector of navSelectors) {
        nav = document.querySelector(selector);
        if (nav) break;
    }

    if (!nav) return;

    nav.innerHTML = `
        <a href="index.html" class="hover:text-white transition-colors">Home</a>
        <a href="benchmark.html" class="hover:text-white transition-colors">Benchmark</a>
        <a href="advanced-features.html" class="hover:text-white transition-colors">Features</a>
        <a href="api.html" class="hover:text-white transition-colors">API Docs</a>
        <a href="manuals/" class="hover:text-white transition-colors">Manuals</a>
    `;
}

// Load menu on page load
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', loadMenu);
} else {
    loadMenu();
}
