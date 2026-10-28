/* Debian Minerva — minimal DOM shim so shell modules load under Node.
 * Only what icons.js / static checks touch: element factory, classList,
 * dataset, attribute bag, child list. NOT a browser — tests that need
 * layout or events must run in a real browser.
 */
'use strict';

function makeClassList(el) {
  var set = {};
  return {
    add: function (c) { set[c] = true; },
    remove: function (c) { delete set[c]; },
    toggle: function (c, on) { if (on === undefined) { if (set[c]) delete set[c]; else set[c] = true; } else if (on) set[c] = true; else delete set[c]; },
    contains: function (c) { return !!set[c]; },
    _set: set
  };
}

function makeElement(tag) {
  var el = {
    tagName: String(tag || 'div').toUpperCase(),
    children: [],
    style: {},
    dataset: {},
    attributes: {},
    _innerHTML: '',
    textContent: '',
    value: '',
    parentNode: null,
    offsetParent: null,
    offsetLeft: 0,
    offsetWidth: 0,
    setAttribute: function (k, v) { this.attributes[k] = String(v); },
    getAttribute: function (k) { return k in this.attributes ? this.attributes[k] : null; },
    removeAttribute: function (k) { delete this.attributes[k]; },
    appendChild: function (c) { c.parentNode = this; this.children.push(c); return c; },
    removeChild: function (c) {
      var i = this.children.indexOf(c);
      if (i !== -1) this.children.splice(i, 1);
      c.parentNode = null; return c;
    },
    querySelector: function () { return null; },
    querySelectorAll: function () { return []; },
    getBoundingClientRect: function () { return { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 }; },
    addEventListener: function () {},
    removeEventListener: function () {},
    focus: function () {},
    blur: function () {},
    closest: function () { return null; },
    isConnected: true
  };
  el.classList = makeClassList(el);
  Object.defineProperty(el, 'innerHTML', {
    get: function () { return this._innerHTML; },
    set: function (v) { this._innerHTML = String(v); this.children = []; }
  });
  Object.defineProperty(el, 'firstChild', {
    get: function () { return this.children[0] || null; }
  });
  return el;
}

function makeDocument() {
  var byId = {};
  var doc = {
    documentElement: makeElement('html'),
    body: makeElement('body'),
    head: makeElement('head'),
    title: '',
    createElement: makeElement,
    createTextNode: function (t) { var e = makeElement('#text'); e.textContent = t; return e; },
    getElementById: function (id) { return byId[id] || null; },
    querySelector: function () { return null; },
    querySelectorAll: function () { return []; },
    addEventListener: function () {},
    removeEventListener: function () {},
    _register: function (id, el) { byId[id] = el; }
  };
  doc.documentElement.dataset = {};
  return doc;
}

function makeLocalStorage() {
  var store = {};
  return {
    getItem: function (k) { return k in store ? store[k] : null; },
    setItem: function (k, v) { store[k] = String(v); },
    removeItem: function (k) { delete store[k]; },
    clear: function () { store = {}; }
  };
}

// Build one shared "window" object good enough for module loading.
function makeWindow() {
  var win = {
    document: makeDocument(),
    localStorage: makeLocalStorage(),
    navigator: { userAgent: 'minerva-shell-test' },
    location: { hash: '', href: 'http://test/' },
    innerWidth: 1280,
    innerHeight: 800,
    addEventListener: function () {},
    removeEventListener: function () {},
    requestAnimationFrame: function (fn) { return setTimeout(fn, 0); },
    cancelAnimationFrame: function (id) { clearTimeout(id); },
    setInterval: setInterval,
    clearInterval: clearInterval,
    setTimeout: setTimeout,
    clearTimeout: clearTimeout,
    matchMedia: function () { return { matches: false, addEventListener: function () {} }; }
  };
  win.window = win;
  win.globalThis = win;
  return win;
}

module.exports = { makeElement: makeElement, makeDocument: makeDocument, makeLocalStorage: makeLocalStorage, makeWindow: makeWindow };
