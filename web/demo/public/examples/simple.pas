uses System.SysUtils;

const A = 0;

procedure Foo; begin Bar('', 0); end;

type A = (
aa, ab, 

accc);

type A = ( // force break
aa, ab
);

type TRec = record A: string[255]; B: array[0
..1] of record AA: Integer; BB: Double    ; end; end;

const Rec: TRec = (A: '';
  B: ((AA: 0; // force break
   BB: 1.0), (AA: 0; BB: 1.0)));

const Precedence = Length([// force break
0,1,2,3 * 2])
 + High(Integer) shr 2 + 1 *
 Low(Integer);


const Precedence2 = Length([// force break
0,1,2,3 * 2] + [10]);

procedure ForwardedDecl; forward;
// Not a nested procedure, because the last decl was forwarded.
procedure CallProc(I: Integer; P: TProc<Integer>); begin P(I); end;

procedure AnonymousRoutines;
procedure NestedProc; begin end;
var A: TProc<Integer>;
begin
  A := procedure(I: Integer) begin Inc(I); end;
  A := procedure(I: Integer)
  begin  // force break
Inc(I); end;

  // nested
  A := procedure(I: Integer)
  begin 
    Inc(I); 
    CallProc(1, procedure(I: Integer) // force break
    begin if True then Inc(I); end);
  end;
end;

const S1 = '''
multiline string
''';

const S2 = '''''
'''
''''';
