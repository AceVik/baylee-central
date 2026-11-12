# The words of the mails Baylee Central sends (docs/email.md), in Simplified Chinese.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = 你的码
code-secret = 请勿把这个码告诉别人。Baylee 的任何人都不会向你索要它。
footer = Baylee 绝不会通过电子邮件向你索要密码。你收到这封邮件，是因为有人为一个 Baylee 账号填写了这个地址。

# The code that confirms an address.
confirm-subject = 确认你在 Baylee 的电子邮件地址
confirm-title = 确认你的电子邮件地址
confirm-intro = 有人为一个 Baylee 账号填写了这个地址。要确认它，请在填写地址的地方输入这个码。
confirm-valid = 这个码的有效期为 { $minutes } 分钟。
confirm-ignore = 如果不是你本人操作的，请忽略这封邮件。没有这个码，这个地址不会被使用。

# The reset code. `name` is the account's sign-in name.
reset-subject = 你的 Baylee 重置码
reset-title = 设置新密码
reset-intro = 有人为 Baylee 账号 { $name } 请求了一个重置码。
reset-how = 在重置页面输入这个码，即可设置新密码。
reset-valid = 这个码只能使用一次，有效期为 { $minutes } 分钟。
reset-link = 打开重置页面
reset-ignore = 如果不是你本人操作的，请忽略这封邮件。你的密码保持不变。

# The notice to an address that another one replaced.
replaced-subject = 你在 Baylee 的电子邮件地址已被替换
replaced-title = 这个地址已被替换
replaced-body = 这个地址不再是某个 Baylee 账号的电子邮件地址。另一个地址已被确认，取代了它。

# The notice to an address that was removed.
removed-subject = 你在 Baylee 的电子邮件地址已被移除
removed-title = 这个地址已被移除
removed-body = 这个地址不再是某个 Baylee 账号的电子邮件地址。它已从该账号中移除。

# Both notices.
notice-check = 如果不是你本人操作的，请登录并查看你账号的安全页面。
notice-link = 打开安全页面

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
