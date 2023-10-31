#!/bin/bash

gperf -m 100 "$@" <<EOF
%ignore-case
%%
absolute
abstract
add
align
and
array
as
asm
assembler
at
automated
begin
case
cdecl
class
const
constructor
contains
default
delayed
deprecated
destructor
dispid
dispinterface
div
do
downto
dynamic
else
end
except
experimental
export
exports
external
far
file
final
finalization
finally
for
forward
function
goto
helper
if
implementation
implements
in
index
inherited
initialization
inline
interface
is
label
library
local
message
mod
name
near
nil
nodefault
not
object
of
on
operator
or
out
overload
override
package
packed
pascal
platform
private
procedure
program
property
protected
public
published
raise
read
readonly
record
reference
register
reintroduce
remove
repeat
requires
resident
resourcestring
safecall
sealed
set
shl
shr
static
stdcall
stored
strict
string
then
threadvar
to
try
type
unit
unsafe
until
uses
var
varargs
variant
virtual
while
with
write
writeonly
xor
EOF