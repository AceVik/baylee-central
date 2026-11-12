# The words of the mails Baylee Central sends (docs/email.md), in Russian.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = Ваш код
code-secret = Никому не передавайте этот код. В Baylee его никто не спрашивает.
footer = Baylee никогда не спрашивает ваш пароль по почте. Вы получили это письмо, потому что этот адрес был указан для аккаунта Baylee.

# Код, который подтверждает адрес.
confirm-subject = Подтвердите адрес электронной почты для Baylee
confirm-title = Подтвердите адрес электронной почты
confirm-intro = Этот адрес был указан для аккаунта Baylee. Чтобы подтвердить его, введите этот код там, где вы указывали адрес.
confirm-valid = Срок действия кода: { $minutes } мин.
confirm-ignore = Если это были не вы, проигнорируйте это письмо. Без кода адрес не используется.

# Код сброса. `name` — имя для входа в аккаунт.
reset-subject = Ваш код сброса для Baylee
reset-title = Задайте новый пароль
reset-intro = Для аккаунта Baylee { $name } запрошен код сброса.
reset-how = Введите код на странице сброса, чтобы задать новый пароль.
reset-valid = Код действует один раз. Срок действия: { $minutes } мин.
reset-link = Открыть страницу сброса
reset-ignore = Если это были не вы, проигнорируйте это письмо. Ваш пароль останется прежним.

# Уведомление адресу, который заменили другим.
replaced-subject = Адрес электронной почты в Baylee заменён
replaced-title = Этот адрес заменён
replaced-body = Этот адрес больше не является адресом электронной почты аккаунта Baylee. Вместо него подтверждён другой адрес.

# Уведомление адресу, который удалили.
removed-subject = Адрес электронной почты в Baylee удалён
removed-title = Этот адрес удалён
removed-body = Этот адрес больше не является адресом электронной почты аккаунта Baylee. Он удалён из аккаунта.

# Оба уведомления.
notice-check = Если это были не вы, войдите и посмотрите страницу безопасности вашего аккаунта.
notice-link = Открыть страницу безопасности

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

# English for now: to be translated.
# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = Confirm address
