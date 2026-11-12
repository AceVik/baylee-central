# Not translated yet: these are the English words, line for line
# (locales/en/mail.ftl), until this language has its own.
#
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
confirm-ignore = If this was not you, ignore this mail. The address is not used without the code.

# The reset code. `name` is the account's sign-in name.
reset-subject = Your reset code for Baylee
reset-title = Set a new password
reset-intro = A reset code was asked for the Baylee account { $name }.
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
