# The words of the mails Baylee Central sends (docs/email.md), in Spanish.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = Tu código
code-secret = No compartas este código. Nadie de Baylee te lo pide.
footer = Baylee nunca te pide la contraseña por correo electrónico. Recibes este correo porque esta dirección se indicó para una cuenta de Baylee.

# The code that confirms an address.
confirm-subject = Confirma tu dirección de correo electrónico para Baylee
confirm-title = Confirma tu dirección de correo electrónico
confirm-intro = Esta dirección se indicó para una cuenta de Baylee. Para confirmarla, escribe este código donde indicaste la dirección.
confirm-valid = El código vale { $minutes } minutos.
confirm-ignore = Si no fuiste tú, ignora este correo. Sin el código, la dirección no se usa.

# The reset code. `name` is the account's sign-in name.
reset-subject = Tu código de restablecimiento para Baylee
reset-title = Establecer una contraseña nueva
reset-intro = Se pidió un código de restablecimiento para la cuenta de Baylee { $name }.
reset-how = Escribe el código en la página de restablecimiento para establecer una contraseña nueva.
reset-valid = El código vale una sola vez, durante { $minutes } minutos.
reset-link = Abrir la página de restablecimiento
reset-ignore = Si no fuiste tú, ignora este correo. Tu contraseña sigue como está.

# The notice to an address that another one replaced.
replaced-subject = Tu dirección de correo electrónico en Baylee se ha sustituido
replaced-title = Esta dirección se ha sustituido
replaced-body = Esta dirección ya no es la dirección de correo electrónico de una cuenta de Baylee. En su lugar se confirmó otra dirección.

# The notice to an address that was removed.
removed-subject = Tu dirección de correo electrónico en Baylee se ha eliminado
removed-title = Esta dirección se ha eliminado
removed-body = Esta dirección ya no es la dirección de correo electrónico de una cuenta de Baylee. Se eliminó de la cuenta.

# Both notices.
notice-check = Si no fuiste tú, inicia sesión y revisa la página de seguridad de tu cuenta.
notice-link = Abrir la página de seguridad

# The notices to the account's confirmed address that its login changed.
# Not translated yet: English until a translation replaces these lines.
reset-done-subject = The password of your Baylee account was reset
reset-done-title = A new password was set
reset-done-body = A reset code was used to set a new password for the Baylee account of this address.
reset-done-kept = The account's data was kept.
reset-done-new = The account starts with an empty vault. Its data from before cannot be opened any more.
reset-done-sessions = The account is signed out everywhere: every session it had was ended.
reset-done-not-you = If this was not you, someone else can sign in to your account now. Ask a Baylee administrator for a reset code and set your own password again.
reset-done-recovery-kept = If the account has a recovery code, it still opens the data. Keep it: with it, a reset keeps the data.
reset-done-recovery-new = A recovery code from before this reset opens nothing any more.
password-subject = The password of your Baylee account was changed
password-title = The password was changed
password-body = The password of the Baylee account of this address was changed.
password-sessions = Every other session of the account was signed out.
password-not-you = If this was not you, someone else knows your password. Set a new one with a reset code: ask for one on the reset page, or ask a Baylee administrator.
issued-subject = A reset code was made for your Baylee account
issued-title = An administrator made a reset code
issued-body = A Baylee administrator made a reset code for the Baylee account of this address. With it, a new password can be set for the account.
issued-valid = The code works once, for { $hours } hours. Until it is used, your password works as it did.
issued-not-you = If you did not ask for it, sign in and change your password on the security page: that ends the code. Then tell an administrator.
