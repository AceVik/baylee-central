# The words of the mails Baylee Central sends (docs/email.md), in French.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = Votre code
code-secret = Ne transmettez ce code à personne. Personne chez Baylee ne vous le demande.
footer = Baylee ne vous demande jamais votre mot de passe par e-mail. Vous recevez cet e-mail parce que cette adresse a été indiquée pour un compte Baylee.

# The code that confirms an address.
confirm-subject = Confirmez votre adresse e-mail pour Baylee
confirm-title = Confirmez votre adresse e-mail
confirm-intro = Cette adresse a été indiquée pour un compte Baylee. Pour la confirmer, saisissez ce code là où vous avez indiqué l’adresse.
confirm-valid = Le code est valable { $minutes } minutes.
confirm-ignore = Si ce n’était pas vous, ignorez cet e-mail. Sans le code, l’adresse n’est pas utilisée.

# The reset code. `name` is the account's sign-in name.
reset-subject = Votre code de réinitialisation pour Baylee
reset-title = Définir un nouveau mot de passe
reset-intro = Un code de réinitialisation a été demandé pour le compte Baylee { $name }.
reset-how = Saisissez le code sur la page de réinitialisation pour définir un nouveau mot de passe.
reset-valid = Le code ne sert qu’une fois, pendant { $minutes } minutes.
reset-link = Ouvrir la page de réinitialisation
reset-ignore = Si ce n’était pas vous, ignorez cet e-mail. Votre mot de passe reste inchangé.

# The notice to an address that another one replaced.
replaced-subject = Votre adresse e-mail chez Baylee a été remplacée
replaced-title = Cette adresse a été remplacée
replaced-body = Cette adresse n’est plus l’adresse e-mail d’un compte Baylee. Une autre adresse a été confirmée à sa place.

# The notice to an address that was removed.
removed-subject = Votre adresse e-mail chez Baylee a été retirée
removed-title = Cette adresse a été retirée
removed-body = Cette adresse n’est plus l’adresse e-mail d’un compte Baylee. Elle a été retirée du compte.

# Both notices.
notice-check = Si ce n’était pas vous, connectez-vous et consultez la page de sécurité de votre compte.
notice-link = Ouvrir la page de sécurité

# The notices to the account's confirmed address that its login changed.
# Not translated yet: English until a translation replaces these lines.
reset-done-subject = The password of your Baylee account was reset
reset-done-title = A new password was set
reset-done-body = A reset code was used to set a new password for the Baylee account of this address.
reset-done-kept = The account's data was kept.
reset-done-new = The account starts with an empty vault. Its data from before cannot be opened any more.
reset-done-sessions = The account is signed out everywhere: every session it had was ended.
reset-done-not-you = If this was not you, someone else can sign in to your account now. Ask a Baylee administrator for a reset code and set your own password again.
reset-done-recovery-kept = Your recovery code still opens your data. Keep it: with it, a reset keeps the data.
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
