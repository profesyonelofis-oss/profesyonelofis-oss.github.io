# Local Services Template: "Anchor Plumbing & Drain"

A trust-first one-page site for a plumber, electrician, HVAC tech, locksmith, roofer, or any local trade. Built to make the phone ring.

**Design:** navy and safety-yellow, Barlow Semi Condensed display caps paired with Barlow, hazard-stripe divider, trust badges everywhere, and a quote form above the fold. Utilitarian and proud of it.

**Sections:** 24/7 emergency top bar, sticky nav, hero with click-to-call and a working quote-form card, six service cards with inline SVG icons and from-prices, four-point "why us" list, service area with a map placeholder, review wall, footer with license number line. Includes Plumber JSON-LD structured data (change the `@type` to Electrician, HVACBusiness, Locksmith, or RoofingContractor to match your trade).

**The quote form** currently shows a friendly alert. Point it at Formspree, Basin, or Netlify Forms (one attribute change) to receive real leads by email.

**Customize in 10 minutes:**
1. Open `index.html` in any text editor.
2. Search for "Anchor" and replace with your company name; search for `555` to update phone numbers (there are several click-to-call links).
3. Edit the six service cards, neighborhoods, reviews, and the license number in the footer.
4. Replace the map placeholder with a Google Maps embed iframe.
5. Colors and fonts live in the `tailwind.config` block near the top. `safety` is the accent color.
