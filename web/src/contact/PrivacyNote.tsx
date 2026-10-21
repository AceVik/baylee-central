// What the form keeps, beside the button, as the privacy notice says it.

import { T, type Elems } from "../i18n";
import { Link, PATHS } from "../router";

const MORE: Elems = { more: (words) => <Link to={`${PATHS.privacy}#contact`}>{words}</Link> };

export function PrivacyNote() {
  return (
    <p className="privacy-note">
      <T id="form.privacy" elems={MORE} />
    </p>
  );
}
