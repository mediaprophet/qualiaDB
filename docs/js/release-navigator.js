/* Filters only the static catalogue; destination runtimes load on their own pages. */
(() => {
  const form = document.getElementById('demo-filters');
  if (!form) return;
  const search = document.getElementById('demo-search');
  const capability = document.getElementById('demo-capability');
  const type = document.getElementById('demo-type');
  const count = document.getElementById('demo-count');
  const empty = document.getElementById('demo-empty');
  const cards = [...document.querySelectorAll('.demo-card')].map(element => ({
    element,
    capability: element.dataset.capability,
    type: element.dataset.type,
    text: element.textContent.toLocaleLowerCase('en'),
  }));

  function filter() {
    const words = search.value.trim().toLocaleLowerCase('en').split(/\s+/).filter(Boolean);
    let visible = 0;
    const tracks = new Set();
    for (const card of cards) {
      const matches = (capability.value === 'all' || capability.value === card.capability)
        && (type.value === 'all' || type.value === card.type)
        && words.every(word => card.text.includes(word));
      card.element.hidden = !matches;
      if (matches) { visible++; tracks.add(card.capability); }
    }
    count.textContent = `${visible} ${visible === 1 ? 'destination' : 'destinations'} · ${tracks.size} capability ${tracks.size === 1 ? 'track' : 'tracks'}`;
    empty.hidden = visible > 0;
  }

  form.hidden = false;
  form.addEventListener('submit', event => event.preventDefault());
  form.addEventListener('input', filter);
  form.addEventListener('change', filter);
  form.addEventListener('reset', () => {
    // A reset event fires before the browser restores the default control values.
    requestAnimationFrame(() => { filter(); search.focus(); });
  });
  filter();
})();
