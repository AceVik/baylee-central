# Baylee Central Source-Available License 1.0

Copyright © 2026 AceVik (the "Licensor"). All rights reserved, except as
granted below. Contact: the operator information at <https://baylee.app>.

This license covers every file in this repository except the parts named in
§ 6, which keep their own license. "Software" means those covered files and
any build of them.

## 1. Why the source is public

The source is published so that anyone can read exactly how Baylee Central
processes their data. Publication grants no right beyond § 2. Reading is
free; using the Software is not.

## 2. What you may do

1. **Read.** You may view, read and study the Software, including through
   a web browser, a code-hosting service's own functions (on GitHub, viewing
   and forking under GitHub's terms; a fork grants no right beyond this
   license), or a local clone kept for reading.
2. **Check our claims.** You may build and run the Software, unmodified,
   on systems you control and that no third party can reach (continuous
   integration runners you use for this included), solely to
   verify its security, privacy or correctness, and you may report what you
   find to the Licensor. You may publish your findings after the Licensor
   has had a reasonable time (at least 90 days) to respond, quoting no more
   of the Software than needed to explain them.
3. **Test against it.** You may build and run the Software, unmodified, on
   systems you control (continuous integration runners included), solely to
   develop and test software that interoperates with the official Baylee
   Central service, including the test suites of the Baylee project
   (<https://github.com/AceVik/baylee>).
4. **Propose changes.** You may prepare changes solely to submit them to
   the official repository. By submitting a contribution you grant the
   Licensor a perpetual, worldwide, royalty-free, irrevocable, transferable
   and sublicensable right to use, change and license it under any terms,
   and you confirm that you may grant that right.
5. **What the law allows anyway.** Nothing in this license restricts what
   the law permits without the Licensor's consent, such as the uses in
   §§ 69d and 69e of the German Copyright Act.

## 3. What you may not do

Except as § 2 permits, or as the Licensor allows in writing, you may not
copy, modify, translate, adapt, distribute, publish, sublicense, sell, rent,
host or offer the Software, or any part of it or work derived from it, as a
service or in any other way, nor run it in production or for anyone but
yourself. In particular you may not operate a service that imitates,
replaces or presents itself as Baylee Central.

## 4. Text and data mining

The Licensor reserves the use of the Software for text and data mining,
including the training of machine-learning models, within the meaning of
§ 44b(3) of the German Copyright Act (UrhG) and Article 4(3) of Directive
(EU) 2019/790. The reservation is declared here and, for the website at
baylee.app, also in machine-readable form (`robots.txt` and
`/.well-known/tdmrep.json`).

## 5. Earlier versions

Versions of this repository published before the commit that added this
file were licensed under the GNU Affero General Public License v3.0 only.
Whoever received such a version keeps the rights that license gave for that
version. This license applies to every later version.

## 6. Parts under other licenses

- `crates/central-api`, `crates/central-client` and `crates/central-crypto`
  are the client libraries the Baylee game client links. They remain
  licensed under the GNU Affero General Public License v3.0 only (`LICENSE-AGPL`),
  so the Baylee project can distribute its software.
- Third-party components keep their own licenses.

## 7. No warranty, limited liability

The Software is provided free of charge and "as is". The Licensor is liable
only for intent and gross negligence (§§ 521, 524 of the German Civil Code
apply accordingly), and without limitation for injury to life, body or
health and under the Product Liability Act.

## 8. Termination

Any use outside this license ends your rights under it automatically. The
Licensor may reinstate them in writing.

## 9. Law and venue

German law governs this license, excluding its conflict-of-laws rules. For
merchants, legal entities under public law and anyone without a general
place of jurisdiction in Germany, the courts at the Licensor's seat have
exclusive jurisdiction.
