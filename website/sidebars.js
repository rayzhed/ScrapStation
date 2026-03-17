/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  mainSidebar: [
    {
      type: 'doc',
      id: 'index',
      label: 'Home',
    },
    'getting-started',
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: [
        'reference/identity',
        'reference/urls-pagination',
        'reference/selectors-html',
        'reference/json-api',
        'reference/field-mapping',
        'reference/transformations',
        'reference/metadata-extraction',
        'reference/tags',
        'reference/detail-page',
        'reference/settings-ui',
        'reference/authentication',
        'reference/hosts',
        'reference/link-resolution',
        'reference/navigation-paths',
        'reference/notices',
      ],
    },
  ],
};

module.exports = sidebars;
