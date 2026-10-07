## Default Permission

内核插件默认权限：允许前端调用统一 IPC 分发命令 kernel_dispatch（含内核保留命令 get_enabled/set_enabled/startup_report）

#### This default permission set includes the following:

- `allow-kernel-dispatch`

## Permission Table

<table>
<tr>
<th>Identifier</th>
<th>Description</th>
</tr>


<tr>
<td>

`kernel:allow-kernel-dispatch`

</td>
<td>

Enables the kernel_dispatch command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`kernel:deny-kernel-dispatch`

</td>
<td>

Denies the kernel_dispatch command without any pre-configured scope.

</td>
</tr>
</table>
