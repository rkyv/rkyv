// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="rkyv.html">rkyv</a></li><li class="chapter-item expanded affix "><li class="part-title">Foundations</li><li class="chapter-item expanded "><a href="motivation.html"><strong aria-hidden="true">1.</strong> Motivation</a></li><li class="chapter-item expanded "><a href="zero-copy-deserialization.html"><strong aria-hidden="true">2.</strong> Zero-copy deserialization</a></li><li class="chapter-item expanded "><a href="architecture.html"><strong aria-hidden="true">3.</strong> Architecture</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="architecture/relative-pointers.html"><strong aria-hidden="true">3.1.</strong> Relative pointers</a></li><li class="chapter-item expanded "><a href="architecture/archive.html"><strong aria-hidden="true">3.2.</strong> Archive</a></li><li class="chapter-item expanded "><a href="architecture/serialize.html"><strong aria-hidden="true">3.3.</strong> Serialize</a></li><li class="chapter-item expanded "><a href="architecture/deserialize.html"><strong aria-hidden="true">3.4.</strong> Deserialize</a></li></ol></li><li class="chapter-item expanded "><a href="format.html"><strong aria-hidden="true">4.</strong> Format</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="format/alignment.html"><strong aria-hidden="true">4.1.</strong> Alignment</a></li></ol></li><li class="chapter-item expanded "><a href="derive-macro-features.html"><strong aria-hidden="true">5.</strong> Derive macro features</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="derive-macro-features/wrapper-types.html"><strong aria-hidden="true">5.1.</strong> Wrapper types</a></li><li class="chapter-item expanded "><a href="derive-macro-features/remote-derive.html"><strong aria-hidden="true">5.2.</strong> Remote derive</a></li></ol></li><li class="chapter-item expanded "><a href="shared-pointers.html"><strong aria-hidden="true">6.</strong> Shared pointers</a></li><li class="chapter-item expanded "><a href="unsized-types.html"><strong aria-hidden="true">7.</strong> Unsized types</a></li><li class="chapter-item expanded "><a href="trait-objects.html"><strong aria-hidden="true">8.</strong> Trait objects</a></li><li class="chapter-item expanded "><a href="validation.html"><strong aria-hidden="true">9.</strong> Validation</a></li><li class="chapter-item expanded "><a href="allocation-tracking.html"><strong aria-hidden="true">10.</strong> Allocation tracking</a></li><li class="chapter-item expanded "><a href="feature-comparison.html"><strong aria-hidden="true">11.</strong> Feature comparison</a></li><li class="chapter-item expanded "><a href="faq.html"><strong aria-hidden="true">12.</strong> FAQ</a></li><li class="chapter-item expanded affix "><a href="contributors.html">Contributors</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
