// Sidebar component

interface NavGroup {
  id: string
  title: string
  items: NavItem[]
}

interface NavItem {
  id: string
  title: string
  path: string
  badge?: string
}

const navGroups: NavGroup[] = [
  {
    id: 'getting-started',
    title: 'Getting Started',
    items: [
      { id: 'quickstart', title: 'Quick Start', path: '/docs/getting-started' },
      { id: 'installation', title: 'Installation', path: '/docs/installation' },
      { id: 'playground', title: 'Playground', path: '/playground' },
    ],
  },
  {
    id: 'products',
    title: 'Products',
    items: [
      { id: 'sigil', title: 'Sigil Language', path: '/docs/sigil', badge: 'beta' },
      { id: 'qliphoth', title: 'Qliphoth', path: '/docs/qliphoth', badge: 'alpha' },
      { id: 'leviathan', title: 'Leviathan', path: '/docs/leviathan', badge: 'stable' },
      { id: 'nyx', title: 'Nyx', path: '/docs/nyx', badge: 'beta' },
    ],
  },
  {
    id: 'reference',
    title: 'Reference',
    items: [
      { id: 'api', title: 'API Reference', path: '/api' },
      { id: 'guides', title: 'Guides', path: '/guides' },
      { id: 'examples', title: 'Examples', path: '/examples' },
    ],
  },
]

export function renderSidebar(currentPath?: string): string {
  return `
    <aside class="sidebar" data-testid="sidebar">
      <nav class="sidebar-nav" data-testid="sidebar-nav">
        ${navGroups.map(group => renderNavGroup(group, currentPath)).join('')}
      </nav>
    </aside>
  `
}

function renderNavGroup(group: NavGroup, currentPath?: string): string {
  const isExpanded = group.items.some(item => currentPath?.startsWith(item.path))

  return `
    <div class="nav-group ${isExpanded ? 'nav-group--expanded' : ''}" data-testid="nav-group-${group.id}">
      <button class="nav-group-header" data-testid="nav-group-header-${group.id}">
        <span class="nav-group-title">${group.title}</span>
        <span class="nav-group-chevron">${isExpanded ? '▼' : '▶'}</span>
      </button>
      <div class="nav-group-content" data-testid="nav-group-content-${group.id}">
        ${group.items.map(item => renderNavItem(item, currentPath)).join('')}
      </div>
    </div>
  `
}

function renderNavItem(item: NavItem, currentPath?: string): string {
  const isActive = currentPath === item.path || currentPath?.startsWith(item.path + '/')

  return `
    <a
      href="${item.path}"
      class="sidebar-link ${isActive ? 'sidebar-link--active' : ''}"
      data-testid="sidebar-link-${item.id}"
    >
      <span class="sidebar-link-text">${item.title}</span>
      ${item.badge ? `<span class="badge badge--${item.badge}">${item.badge}</span>` : ''}
    </a>
  `
}
