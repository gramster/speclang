IRModule     := "module" QName "{" { IRItem } "}" ;

IRItem       := IRTypeDef | IRCapDef | IRExtern | IRFunc ;

IRCapDef     := "cap" Ident [ "(" { CapField } ")" ] ";" ;
CapField     := Ident ":" Type ;

IRTypeDef    := "type" Ident "=" Type ";" ;

IRExtern     := "extern" "fn" Ident "(" [ Params ] ")" "->" Type
                [ "effects" "(" [ EffectSig ] ")" ] ";" ;

IRFunc       := "fn" Ident "(" [ Params ] ")" "->" Type
                [ "effects" "(" [ EffectSig ] ")" ]
                [ ContractMeta ]
                Block ;

Params       := Param { "," Param } ;
Param        := Ident ":" Type ;

EffectSig    := Ident { "," Ident } ;  # capability types required

ContractMeta := { "@requires" Pred ";" | "@ensures" Pred ";" } ;

Block        := "{" { Stmt } "}" ;

Stmt         := LetStmt | AssignStmt | IfStmt | MatchStmt | ReturnStmt
              | AssertStmt | ExprStmt ;

LetStmt      := "let" Ident ":" Type "=" Expr ";" ;
AssignStmt   := Ident "=" Expr ";" ;

IfStmt       := "if" Expr Block "else" Block ;
MatchStmt    := "match" Expr "{" { MatchArm } "}" ;
MatchArm     := Pattern "=>" Block ;

ReturnStmt   := "return" Expr ";" ;
AssertStmt   := "assert" "(" Pred "," String ")" ";" ;
ExprStmt     := Expr ";" ;

Expr         := Literal | Ident | Call | BinOp | UnOp
              | StructLit | FieldGet | EnumLit | TupleLit
              | IfExpr | MatchExpr ;

# Pred is a pure boolean Expr subset
Pred         := Expr ;

Type         := Prim
              | "struct" "{" { Field } "}"
              | "enum" "{" { Variant } "}"
              | "own" "[" Region "," Type "]"
              | "ref" "[" Type "]"
              | "mutref" "[" Type "]"
              | "slice" "[" Type "]"
              | "mutslice" "[" Type "]"
              | Ident ;

Region       := "heap" | Ident ;

Call         := "call" QName "(" [ Args ] ")" ;
Args         := Expr { "," Expr } ;