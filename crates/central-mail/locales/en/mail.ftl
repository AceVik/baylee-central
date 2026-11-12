# The words of the mails Baylee Central sends (docs/email.md).
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is
# written. A line that starts with `#` is a comment. No markup: the words go
# into the plain text as they are and into the HTML escaped.

brand = Baylee
code-label = Your code
code-secret = Do not pass this code on. Nobody at Baylee asks for it.
footer = Baylee never asks for your password by e-mail. You get this mail because this address was named for a Baylee account.

# The code that confirms an address.
confirm-subject = Confirm your e-mail address for Baylee
confirm-title = Confirm your e-mail address
confirm-intro = This address was named for a Baylee account. To confirm it, enter this code where you named the address.
confirm-valid = The code works for { $minutes } minutes.
confirm-link = Confirm address
confirm-ignore = If this was not you, ignore this mail. The address is not used without the code.

# The reset code. `name` is the account's sign-in name.
reset-subject = Your reset code for Baylee
reset-title = Set a new password
reset-intro = A reset code was requested for the Baylee account { $name }.
reset-how = Enter the code on the reset page to set a new password.
reset-valid = The code works once, for { $minutes } minutes.
reset-link = Open the reset page
reset-ignore = If this was not you, ignore this mail. Your password stays as it is.

# The notice to an address that another one replaced.
replaced-subject = Your e-mail address on Baylee was replaced
replaced-title = This address was replaced
replaced-body = This address is no longer the e-mail address of a Baylee account. Another address was confirmed in its place.

# The notice to an address that was removed.
removed-subject = Your e-mail address on Baylee was removed
removed-title = This address was removed
removed-body = This address is no longer the e-mail address of a Baylee account. It was removed from the account.

# Both notices.
notice-check = If this was not you, sign in and look at the security page of your account.
notice-link = Open the security page

# The notices to the account's confirmed address that its login changed.
# They name no account: the sign-in name is half of what a reset by mail
# needs, and these mails go out without anyone asking for them.

# A reset code set a new password.
reset-done-subject = The password of your Baylee account was reset
reset-done-title = A new password was set
reset-done-body = A reset code was used to set a new password for the Baylee account of this address.
reset-done-kept = The account's data was kept.
reset-done-new = The account starts with an empty vault. Its data from before cannot be opened any more.
reset-done-sessions = The account is signed out everywhere: every session it had was ended.
reset-done-not-you = If this was not you, someone else can sign in to your account now. Ask a Baylee administrator for a reset code and set your own password again.
reset-done-recovery-kept = If the account has a recovery code, it still opens the data. Keep it: with it, a reset keeps the data.
reset-done-recovery-new = A recovery code from before this reset opens nothing any more.

# The password was changed.
password-subject = The password of your Baylee account was changed
password-title = The password was changed
password-body = The password of the Baylee account of this address was changed.
password-sessions = Every other session of the account was signed out.
password-not-you = If this was not you, someone else knows your password. Set a new one with a reset code: ask for one on the reset page, or ask a Baylee administrator.

# An administrator made a reset code. `hours` is how long it works.
issued-subject = A reset code was made for your Baylee account
issued-title = An administrator made a reset code
issued-body = A Baylee administrator made a reset code for the Baylee account of this address. With it, a new password can be set for the account.
issued-valid = The code works once, for { $hours } hours. Until it is used, your password works as it did.
issued-not-you = If you did not ask for it, sign in and change your password on the security page: that ends the code. Then tell an administrator.
