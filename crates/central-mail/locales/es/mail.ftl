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
reset-done-subject = Se restableció la contraseña de tu cuenta de Baylee
reset-done-title = Se estableció una contraseña nueva
reset-done-body = Se usó un código de restablecimiento para establecer una contraseña nueva en la cuenta de Baylee de esta dirección.
reset-done-kept = Los datos de la cuenta se conservaron.
reset-done-new = La cuenta empieza con una bóveda vacía. Sus datos de antes ya no se pueden abrir.
reset-done-sessions = Se cerró la sesión de la cuenta en todas partes: terminaron todas las sesiones que tenía.
reset-done-not-you = Si no fuiste tú, otra persona puede iniciar sesión ahora en tu cuenta. Pide un código de restablecimiento a un administrador de Baylee y establece de nuevo tu propia contraseña.
reset-done-recovery-kept = Si la cuenta tiene un código de recuperación, sigue abriendo los datos. Guárdalo: con él, un restablecimiento conserva los datos.
reset-done-recovery-new = Un código de recuperación anterior a este restablecimiento ya no abre nada.
password-subject = Se cambió la contraseña de tu cuenta de Baylee
password-title = Se cambió la contraseña
password-body = Se cambió la contraseña de la cuenta de Baylee de esta dirección.
password-sessions = Se cerró la sesión de todas las demás sesiones de la cuenta.
password-not-you = Si no fuiste tú, otra persona conoce tu contraseña. Establece una nueva con un código de restablecimiento: pídelo en la página de restablecimiento o pídeselo a un administrador de Baylee.
issued-subject = Se generó un código de restablecimiento para tu cuenta de Baylee
issued-title = Un administrador generó un código de restablecimiento
issued-body = Un administrador de Baylee generó un código de restablecimiento para la cuenta de Baylee de esta dirección. Con él se puede establecer una contraseña nueva para la cuenta.
issued-valid = El código funciona una sola vez, durante { $hours } horas. Hasta que se use, tu contraseña funciona como antes.
issued-not-you = Si no lo pediste, inicia sesión y cambia tu contraseña en la página de seguridad: eso anula el código. Después díselo a un administrador.

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = Confirmar la dirección
