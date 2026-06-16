// Menu loader for consistent navigation across all QualiaDB documentation pages
// Loads menu.json and renders navigation dynamically with responsive dropdowns

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

    // Group navigation items by their group field
    const groupedItems = {};
    menu.navigation.forEach(item => {
        const group = item.group || 'Other';
        if (!groupedItems[group]) {
            groupedItems[group] = [];
        }
        groupedItems[group].push(item);
    });

    // Build the navigation HTML with dropdowns
    let navHTML = '';

    // Add hamburger menu button (hidden on desktop)
    navHTML += `
        <button id="mobile-menu-btn" class="md:hidden text-gray-300 hover:text-white p-2">
            <i class="fa-solid fa-bars text-xl"></i>
        </button>
    `;

    // Desktop navigation with dropdowns
    navHTML += `<div id="desktop-nav" class="hidden md:flex items-center gap-6 text-sm">`;

    for (const [groupName, items] of Object.entries(groupedItems)) {
        if (items.length === 1) {
            // Single item - render as regular link
            const item = items[0];
            const isActive = item.href === currentPage ? 'nav-active font-medium' : '';
            const highlight = item.highlight ? 'text-cyan-400' : 'hover:text-white';
            const icon = item.icon ? `<i class="${item.icon} mr-1"></i>` : '';
            navHTML += `<a href="${item.href}" class="${highlight} ${isActive} transition-colors flex items-center">${icon}${item.label}</a>`;
        } else {
            // Multiple items - render as dropdown
            navHTML += `
                <div class="relative group">
                    <button class="text-gray-300 hover:text-white transition-colors flex items-center gap-1">
                        ${groupName}
                        <i class="fa-solid fa-chevron-down text-xs"></i>
                    </button>
                    <div class="absolute left-0 mt-2 w-48 bg-gray-800 rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-50">
                        <div class="py-2">
                            ${items.map(item => {
                                const isActive = item.href === currentPage ? 'nav-active font-medium' : '';
                                const highlight = item.highlight ? 'text-cyan-400' : 'hover:text-white';
                                const icon = item.icon ? `<i class="${item.icon} mr-2"></i>` : '';
                                return `<a href="${item.href}" class="${highlight} ${isActive} block px-4 py-2 transition-colors flex items-center">${icon}${item.label}</a>`;
                            }).join('')}
                        </div>
                    </div>
                </div>
            `;
        }
    }

    navHTML += `</div>`;

    // Mobile navigation (hidden on desktop)
    navHTML += `
        <div id="mobile-nav" class="hidden md:hidden absolute top-full left-0 right-0 bg-gray-900 border-t border-gray-700">
            <div class="py-4 px-4 space-y-2">
    `;

    for (const [groupName, items] of Object.entries(groupedItems)) {
        if (items.length === 1) {
            // Single item - render as regular link
            const item = items[0];
            const isActive = item.href === currentPage ? 'nav-active font-medium' : '';
            const highlight = item.highlight ? 'text-cyan-400' : 'hover:text-white';
            const icon = item.icon ? `<i class="${item.icon} mr-2"></i>` : '';
            navHTML += `<a href="${item.href}" class="${highlight} ${isActive} block px-4 py-2 rounded transition-colors flex items-center">${icon}${item.label}</a>`;
        } else {
            // Multiple items - render as collapsible section
            navHTML += `
                <div class="mobile-dropdown">
                    <button class="mobile-dropdown-btn w-full text-left px-4 py-2 text-gray-300 hover:text-white rounded transition-colors flex items-center justify-between">
                        <span class="flex items-center">
                            <i class="fa-solid fa-folder mr-2"></i>
                            ${groupName}
                        </span>
                        <i class="fa-solid fa-chevron-down text-xs transition-transform"></i>
                    </button>
                    <div class="mobile-dropdown-content hidden pl-4 mt-1 space-y-1">
                        ${items.map(item => {
                            const isActive = item.href === currentPage ? 'nav-active font-medium' : '';
                            const highlight = item.highlight ? 'text-cyan-400' : 'hover:text-white';
                            const icon = item.icon ? `<i class="${item.icon} mr-2"></i>` : '';
                            return `<a href="${item.href}" class="${highlight} ${isActive} block px-4 py-2 rounded transition-colors flex items-center">${icon}${item.label}</a>`;
                        }).join('')}
                    </div>
                </div>
            `;
        }
    }

    navHTML += `
            </div>
        </div>
    `;

    nav.innerHTML = navHTML;

    // Add mobile menu toggle functionality
    const mobileMenuBtn = document.getElementById('mobile-menu-btn');
    const mobileNav = document.getElementById('mobile-nav');

    if (mobileMenuBtn && mobileNav) {
        mobileMenuBtn.addEventListener('click', () => {
            mobileNav.classList.toggle('hidden');
        });

        // Close mobile menu when clicking outside
        document.addEventListener('click', (e) => {
            if (!mobileMenuBtn.contains(e.target) && !mobileNav.contains(e.target)) {
                mobileNav.classList.add('hidden');
            }
        });
    }

    // Add mobile dropdown toggle functionality
    const mobileDropdownBtns = document.querySelectorAll('.mobile-dropdown-btn');
    mobileDropdownBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const content = btn.nextElementSibling;
            const icon = btn.querySelector('.fa-chevron-down');
            content.classList.toggle('hidden');
            icon.classList.toggle('rotate-180');
        });
    });

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
        <button id="mobile-menu-btn" class="md:hidden text-gray-300 hover:text-white p-2">
            <i class="fa-solid fa-bars text-xl"></i>
        </button>
        <div id="desktop-nav" class="hidden md:flex items-center gap-6 text-sm">
            <a href="index.html" class="hover:text-white transition-colors">Home</a>
            <a href="benchmark.html" class="hover:text-white transition-colors">Benchmark</a>
            <a href="advanced-features.html" class="hover:text-white transition-colors">Features</a>
            <a href="api.html" class="hover:text-white transition-colors">API Docs</a>
            <a href="manuals/" class="hover:text-white transition-colors">Manuals</a>
        </div>
        <div id="mobile-nav" class="hidden md:hidden absolute top-full left-0 right-0 bg-gray-900 border-t border-gray-700">
            <div class="py-4 px-4 space-y-2">
                <a href="index.html" class="hover:text-white transition-colors block px-4 py-2 rounded">Home</a>
                <a href="benchmark.html" class="hover:text-white transition-colors block px-4 py-2 rounded">Benchmark</a>
                <a href="advanced-features.html" class="hover:text-white transition-colors block px-4 py-2 rounded">Features</a>
                <a href="api.html" class="hover:text-white transition-colors block px-4 py-2 rounded">API Docs</a>
                <a href="manuals/" class="hover:text-white transition-colors block px-4 py-2 rounded">Manuals</a>
            </div>
        </div>
    `;

    // Add mobile menu toggle functionality
    const mobileMenuBtn = document.getElementById('mobile-menu-btn');
    const mobileNav = document.getElementById('mobile-nav');

    if (mobileMenuBtn && mobileNav) {
        mobileMenuBtn.addEventListener('click', () => {
            mobileNav.classList.toggle('hidden');
        });

        document.addEventListener('click', (e) => {
            if (!mobileMenuBtn.contains(e.target) && !mobileNav.contains(e.target)) {
                mobileNav.classList.add('hidden');
            }
        });
    }
}

// Load menu on page load
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', loadMenu);
} else {
    loadMenu();
}
