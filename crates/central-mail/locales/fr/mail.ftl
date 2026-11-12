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
