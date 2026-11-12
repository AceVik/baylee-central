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
reset-done-subject = 你的 Baylee 账号的密码已被重置
reset-done-title = 已设置新密码
reset-done-body = 有人用重置码为这个地址对应的 Baylee 账号设置了新密码。
reset-done-kept = 账号的数据已保留。
reset-done-new = 账号从空保险库开始。之前的数据已无法再打开。
reset-done-sessions = 账号已在所有地方退出登录：它的每个会话都已结束。
reset-done-not-you = 如果不是你本人操作的，现在别人可以登录你的账号。请向 Baylee 管理员索取重置码，再次设置你自己的密码。
reset-done-recovery-kept = 如果账号有恢复码，它仍能打开数据。请保管好它：有了它，重置后数据会保留。
reset-done-recovery-new = 这次重置之前的恢复码已无法打开任何东西。
password-subject = 你的 Baylee 账号的密码已更改
password-title = 密码已更改
password-body = 这个地址对应的 Baylee 账号的密码已被更改。
password-sessions = 账号的其他所有会话都已退出登录。
password-not-you = 如果不是你本人操作的，说明别人知道你的密码。请用重置码设置新密码：在重置页面索取一个，或向 Baylee 管理员索取。
issued-subject = 已为你的 Baylee 账号生成重置码
issued-title = 管理员生成了重置码
issued-body = Baylee 管理员为这个地址对应的 Baylee 账号生成了一个重置码。凭它可以为该账号设置新密码。
issued-valid = 这个码只能使用一次，有效期为 { $hours } 小时。在它被使用之前，你的密码照常有效。
issued-not-you = 如果不是你要求的，请登录并在安全页面更改你的密码：这会使该码失效。然后告诉管理员。

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = 确认地址
