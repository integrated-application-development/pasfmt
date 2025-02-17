use std::path::Path;

use crate::*;

pub fn generate_test_files(root_dir: &Path) {
    directives::generate(root_dir);
    comments::generate(root_dir);
    anonymous::generate(root_dir);
    import_exports::generate(root_dir);
    type_decls::generate(root_dir);
    prop_decls::generate(root_dir);
    const_records::generate(root_dir);
    routines::generate(root_dir);
    control_flows::generate(root_dir);
    statements::generate(root_dir);
    expressions::generate(root_dir);
    attributes::generate(root_dir);
    line_length_violations::generate(root_dir);
    regression::generate(root_dir);
}

mod directives {
    use super::*;

    pub fn generate(root_dir: &Path) {
        conditional::generate(root_dir);
        compiler::generate(root_dir);
    }

    mod conditional {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                ifdef = "
                    begin
                      A :=
                          B
                    {$ifdef A}
                              + C
                    {$endif}
                          ;
                    end;
                ",
                if_else = "
                    begin
                      A :=
                          B
                    {$ifdef A}
                              + C
                    {$elseif B}
                              + D
                    {$endif}
                          ;
                    end;
                ",
                nested = "
                    begin
                      A :=
                          B
                    {$ifdef A}
                      {$ifdef C}
                              + C
                      {$endif}
                      {$ifdef E}
                              + E
                      {$endif}
                    {$elseif B}
                              + D
                    {$endif}
                          ;
                    end;
                ",
            );
        }
    }

    mod compiler {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                inline = "
                    begin
                      A := B {$C+} + C {$C-};
                      A :=
                          BBB {$C+} + CCCCC {$C-};
                      A :=
                          BBBB {$C+}
                              + CCCCC {$C-};
                    end;
                ",
            );
        }
    }
}

mod comments {
    use super::*;

    pub fn generate(root_dir: &Path) {
        midline_line::generate(root_dir);
        midline_block::generate(root_dir);
        child_lines::generate(root_dir);
        conditional_directives::generate(root_dir);
    }

    mod midline_line {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                end_of_line = "
                    AAAAAAAA := BBBBBBBBB; // CCCC
                    AAAAAAAA :=
                        BBBBBBBBB; // CCCCCCCCCCCC
                ",
                assignment = "
                    AAAAAAAA := // BBBBBBBBB
                        CCCCCCCCC;
                    AAAAAAAA :=
                        BBBBBBBBB // CCCCC
                        ;
                    AAAAAAAA :=
                        BBBBBBBBBB // CCCCC
                        ;
                    AAAAAAAA := //
                        BBBBBBBBB //
                            //
                            + CCCCCCCCC //
                            //
                            + //
                            DDDDDDDDD //
                        ; //
                ",
                invocation = "
                    AAAAAA //
                        ( //
                            CCCCCCCCC
                                //
                            , //
                            //
                            DDDDDDDDD
                        );
                    AAAAAA //
                        (CCCCCCCCC, DDDDDDDDD);
                    AAAAAA //
                        (
                            CCCCCCCCCCC,
                            DDDDDDDDDDD
                        );
                ",
                for_loop = "
                    for //
                        var A //
                            := 0 //
                        to //
                        1 //
                        do
                      ;
                    for //
                        var A //
                        in //
                        B //
                        do
                      ;
                ",
                if_then = "
                    if //
                    (AAA.BBB.CCC
                            and DDDDDDDDDDDDD)
                        or EEEEEEEEEEEEEEEEEE
                            and FFFFFFFFFFFF then
                      ;
                ",
            );
        }
    }

    mod midline_block {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                end_of_line = "
                    AAAAAAAA := BBBBBBBBB; {}
                    AAAAAAAA :=
                        BBBBBBBBB; {      }
                ",
                assignment = "
                    AAAAAAAA := {} CCCCCCCCC;
                    AAAAAAAA := {      }
                        CCCCCCCCC;
                    AAAAAAAA :=
                        {} CCCCCCCCC;
                    AAAAAAAA := BBBBBBBBB {};
                    AAAAAAAA :=
                        BBBBBBBBB {      };
                    AAAAAAAA :=
                        BBBBBBBBB {              };
                    AAAAAAAA := {}
                        BBBBBBBBB {}
                            {}
                            + CCCCCCCCC {}
                            {}
                            + {} DDDDDDDDD {}; {}
                ",
                invocation = "
                    AAAAAA {} ({}
                        CCCCCCCCC
                            {                    }, {}
                        {}
                        DDDDDDDDD
                    );
                    AAAAAA {                     } (
                        CCCCCCCCC,
                        DDDDDDDDD
                    );
                ",
                for_loop = "
                    for {} var A {               } :=
                            0 {}
                        to {} 1 {} do
                      ;
                    for {} var A {               }
                        in {} B {} do
                      ;
                ",
                multi_mid_line = "
                    AAAAAAAA :=
                        {
                        }
                        CCCCCCCCC;
                ",
                if_then = "
                    if {} (AAA.BBB.CCC
                            and DDDDDDDDDDDDD)
                        or EEEEEEEEEEEEEEEEEE
                            and FFFFFFFFFFFF then
                      ;
                ",
            );
        }
    }

    mod child_lines {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                before_first_token = "
                    Foo(
                        procedure
                        begin //
                          ;
                        end
                    );
                ",
                only_token = "
                    Foo(
                        procedure
                        begin //
                        end
                    );
                ",
                if_else = "
                    if AAAAAA then //
                      BBBBBBB;
                    if AAAAAA then //
                    begin
                    end;
                    if AAAAAA then //
                      BBBBBBB
                    else
                      CCCCCCC;
                    if AAAAAA then //
                    begin
                    end
                    else //
                      CCCCCCC;
                    if AAAAAA then //
                      BBBBBBB
                    else //
                    begin
                    end;
                ",
                after_do = "
                    try
                    except
                      on AAAAAAAA do //
                        BBBBBBB;
                      on AAAAAAAA do //
                      begin
                      end;
                    end;
                    while AAAAAAAA do //
                      BBBBBBB;
                    while AAAAAAAA do //
                    begin
                    end;
                    with AAAAAAAA do //
                      BBBBBBB;
                    with AAAAAAAA do //
                    begin
                    end;
                    for AAAAAAAA in BBBBBB do //
                      CCCCCCCC;
                    for AAAAAAAA in BBBBBB do //
                    begin
                    end;
                ",
                after_colon = "
                    case AAAAA of
                      BBBBB: //
                        BBBBBBB;
                      CCCCC: //
                      begin
                      end;
                    end;
                ",
                after_lparen = "
                    type
                      A = record
                      case AAA of
                        BBB: ( //
                          F: FFF;
                        )
                      end;
                ",
            );
        }
    }

    mod conditional_directives {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                after = "
                    Foo(
                    {$ifdef A} //
                        A
                    {$else} //
                        B
                    {$endif} //
                    );
                ",
            );
        }
    }
}

mod anonymous {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            only_arg = "
                    AAAAAAAA(procedure begin end);
                    AAAAAAAAA(
                        procedure begin BBB; end
                    );
                    AAAAAAAAA(
                        procedure
                        begin
                          BBBBBBBB;
                        end
                    );
            ",
            first_arg = "
                    AA(procedure begin end, B, C);
                    AAA(
                        procedure begin B; end,
                        C,
                        D
                    );
                    AAA(
                        procedure
                        begin
                          BBBBBBB;
                        end,
                        C,
                        D
                    );
            ",
            not_first_arg = "
                    A(B, procedure begin end, D);
                    A(
                        B,
                        C,
                        procedure begin DDDD; end,
                        E,
                        F
                    );
                    A(
                        B,
                        C,
                        procedure
                        begin
                          DDDDDDD;
                        end,
                        E,
                        F
                    );
            ",
            arg_list = "
                    AAA(function(A): C begin end);
                    AAAA(
                        function(A): C begin end
                    );
                    AAAA(
                        function(A: B): C
                        begin
                        end
                    );
                    AAAA(
                        function(AAAA: BBBBB):
                            CCCC
                        begin
                        end
                    );
                    AAAA(
                        function(
                            AAAAAAA: BBBBBBB
                        ): CCCC
                        begin
                        end
                    );
            ",
            decl_blocks = "
                    A(
                        procedure
                        var
                          B: C;
                          B: C;
                        begin
                        end
                    );
                    A(
                        procedure
                        label
                          aaaaaaaa, bbbbbbb, cccc;
                        label
                          aaaaaaaa,
                          bbbbbbb,
                          ccccc;
                        begin
                        end
                    );
            ",
            empty_blocks = "
                    aaa(procedure var begin end);
                    begin
                      aaa(
                          procedure var begin end
                      );
                      begin
                        aaa(
                            procedure
                            var
                            begin
                            end
                        );
                      end;
                    end;
                    aa(procedure const begin end);
                    aaa(procedure type begin end);

                    aaa(
                        procedure const begin end
                    );
                    aaa(
                        procedure
                        const
                        begin
                          a;
                        end
                    );

                    aaaa(
                        procedure type begin end
                    );
                    aaaa(
                        procedure
                        type
                        begin
                          a;
                        end
                    );
                    aaa(
                        procedure
                        threadvar
                        begin
                        end
                    );
            ",
            assignment = "
                    A := procedure begin AAA; end;
                    AA :=
                        procedure begin AAA; end;
                    AA :=
                        procedure
                        begin
                          A;
                          A;
                        end;
                    AA :=
                        procedure
                        var
                          A: TProc;
                        begin
                          A;
                        end;
            "
        );
    }
}

mod import_exports {
    use super::*;

    pub fn generate(root_dir: &Path) {
        uses::generate(root_dir);
        requires::generate(root_dir);
        contains::generate(root_dir);
        exports::generate(root_dir);
    }

    mod uses {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                uses = "
                    program A;
                    uses
                      AAAA.AAAA,
                      BBB.BBB,
                      CCCCCCC;
                    uses
                      AAAAA.AAA,
                      BBB.BBB,
                      CCCCCCCC;
                ",
                uses_in = "
                    program A;
                    uses
                      AA.AA in 'a',
                      BBBBBB in 'b';
                    uses
                      AAAAAA in 'a',
                      BB.BBB in 'b';
                    uses
                      AAAAAAAAAAAAAA
                              .AAAAAAAAAAAA
                              .AAAAAAAAAAAA
                          in 'aaaaaaaaaaaaa',
                      BBBBBBB
                          in 'bbbbbbbbbbbbbbbb';
                    uses
                      AAAA.AAAA
                          in 'aaaaaaaaaaaaa',
                      BBBBB in 'b';
                    uses
                      AA.AA in 'A',
                      BBBBBBB
                          in 'bbbbbbbbbbbbbbb';
                ",
            );
        }
    }

    mod requires {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                requires = "
                    package A;
                    requires
                      AAAAAA,
                      BBB.BB,
                      CCCCC;
                    requires
                      AAAAA.AAA,
                      BBBBBBB,
                      CCCCC.CC;
                    requires
                      AAAAAAAAAAAAAA
                          .AAAAAAAAAAAAA,
                      BBBBBBB,
                      CCCCC.CC;
                ",
            );
        }
    }

    mod contains {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                contains = "
                    package A;
                    contains
                      AAAAA,
                      BBB.BB,
                      CCCCCC;
                    contains
                      AAAA.AAAA,
                      BBBBBBB,
                      CCCC.CCC;
                ",
                contains_in = "
                    package A;
                    contains
                      AA.AA in 'a',
                      BBB.BB in 'b';
                    contains
                      AA.AAA in 'a',
                      BBBBBB in 'b';
                    contains
                      AAAA.AAAA
                          in 'aaaaaaaaaaaaa',
                      BBBBB in 'b';
                    contains
                      AAAAAAAAAAAAAA
                              .AAAAAAAAAAAA
                              .AAAAAAAAAAAA
                          in 'aaaaaaaaaaaaa',
                      BBBBBBBBBBBBBB
                          in 'bbbbbbbbbbbbb';
                    contains
                      AA.AA in 'A',
                      BBBBBBBBB
                          in 'bbbbbbbbbbbbb';
                ",
            );
        }
    }

    mod exports {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                ident = "
                    exports
                      AAAAAA,
                      BBBBBB,
                      CCCCCC;
                    exports
                      AAAAAAAA,
                      BBBBBBBB,
                      CCCCCCCC;
                ",
                index = "
                    exports
                      AAAAAAAAA index BBBBBB;
                    exports
                      AA index BB,
                      C index D;
                    exports
                      AAA index BBB,
                      CCC index DDD;
                    exports
                      AAAAAAAAAAA
                          index BBBBBBBBBB,
                      CC index DD;
                    exports
                      AAAAAAAAAA
                          index
                              BBBBBBB + CCCCCCCC,
                      CC index DD;
                    exports
                      AAAAAAAAAA
                          index
                              BBBBBBBB
                                  + CCCCCCCCC,
                      CC index DD;
                ",
                name = "
                    exports
                      AAAAAAAAA name BBBBBBB;
                    exports
                      AA name BB,
                      CC name DD;
                    exports
                      AAAA name BBB,
                      CCCC name DDD;
                    exports
                      AAAAAAAAAAA
                          name BBBBBBBBBBB,
                      CC name DD;
                    exports
                      AAAAAAAAAAA
                          name
                              BBBBBBBB + CCCCCCCC,
                      CC name DD;
                    exports
                      AAAAAAAAAAA
                          name
                              BBBBBBBBB
                                  + CCCCCCCCC,
                      CC name DD;
                ",
                index_name = "
                    exports
                      AAA index AAA name AAA;
                    exports
                      AAAA index AAAA name AAAA,
                      BBBB index BBBB name BBBB;
                    exports
                      AAAAA
                          index AAAAAA
                          name AAAAAA,
                      BBBB index BBBB name BBBB;
                    exports
                      AAAAA
                          index
                              AAAAAAA + AAAAAAAAA
                          name AAAA,
                      BBBB index BBBB name BBBB;
                    exports
                      AAAAA
                          index
                              AAAAAAAA
                                  + AAAAAAAAAA
                          name AAAA,
                      BBBB index BBBB name BBBB;
                ",
            );
        }
    }
}

mod type_decls {
    use super::*;

    pub fn generate(root_dir: &Path) {
        struct_type::generate(root_dir);
        variant_record::generate(root_dir);
        of_types::generate(root_dir);
        procedural::generate(root_dir);
        string::generate(root_dir);
        ordinal::generate(root_dir);
        aliases::generate(root_dir);
        fields::generate(root_dir);
    }

    mod struct_type {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                object = "
                    type
                      A = object
                      end;
                      AAAAAAAAAAAAAAAAAAAA =
                          object
                      end;
                      AAAA<AAA, AAA, AAA> = object
                      end;
                      AAAAA<AAA, AAA, AAA> =
                          object
                      end;
                      AAAAAAAAAAAAAA<
                          AAA,
                          AAA,
                          AAA
                      > = object
                      end;
                ",
                packed_object = "
                    type
                      A = packed object
                      end;
                      AAAAAAAAAAAAA =
                          packed object
                      end;
                ",
                class = "
                    type
                      A = class
                      end;
                      AAAAAAAAAAAAAAAAAAAAA =
                          class
                      end;
                      A = class(BBBBBBB)
                      end;
                      AAAAAAAAAAAA =
                          class(BBBBBBB)
                      end;
                      AAAAAAAAAAAA =
                          class(
                              BBBBB,
                              CCCCC,
                              DDDD
                          )
                      end;
                      AAAAAA<A, B, C> =
                          class(BBBB)
                      end;
                      AAAAAAAAAAAA<
                          AAA,
                          BBB,
                          CCC
                      > = class(BBBB)
                      end;
                      AAAAAAAAAAAA<
                          AAA,
                          BBB,
                          CCC
                      > =
                          class(
                              BBBB,
                              CCCCC,
                              DDDDD
                          )
                      end;
                ",
                packed_class = "
                    type
                      A = packed class
                      end;
                      AAAAAAAAAAAAAAAAAAA =
                          packed class
                      end;
                      AAAAAAAAAAAAAAAAAAAAA =
                          packed class
                      end;
                      A = packed class(BBBBBBB)
                      end;
                      AAAAAAAAAAAA =
                          packed class(BBBBBBB)
                      end;
                ",
                abstract_class = "
                    type
                      A = class abstract
                      end;
                      AAAAAAAAAAAAAAAAAAAA =
                          class abstract
                      end;
                      AAAAAAAAAAAAAAAAAAAAA =
                          class abstract
                      end;
                      A = class abstract(BBBBBBB)
                      end;
                      AAAAAAAAAAAA =
                          class abstract(BBBBBBB)
                      end;
                ",
                sealed_class = "
                    type
                      A = class sealed
                      end;
                      AAAAAAAAAAAAAAAAAAAA =
                          class sealed
                      end;
                      AAAAAAAAAAAAAAAAAAAAA =
                          class sealed
                      end;
                      A = class sealed(BBBBBBB)
                      end;
                      AAAAAAAAAAAA =
                          class sealed(BBBBBBB)
                      end;
                ",
                class_helper = "
                    type
                      A = class helper for BBBBBBB
                      end;
                      AA =
                          class helper for BBBBBBB
                      end;
                      A =
                          class helper for
                              BBBBBBBBB.BBBBBBBBBB
                      end;
                      A =
                          class helper for
                              BBBBBBBBB
                                  .BBBBBBBBBBB
                      end;
                      AA = class helper(BB) for BB
                      end;
                      AAA =
                          class helper(BB) for BB
                      end;
                      AAA =
                          class helper(BBB) for
                              BBBBBBBBBB.BBBBBBBBB
                      end;
                      AAA =
                          class helper(BBB) for
                              BBBBBBBBBB
                                  .BBBBBBBBBB
                      end;
                      AAA =
                          class helper(
                              BBBBBBB
                          ) for BBBBBBBBBB
                      end;
                      AAA =
                          class helper(
                              BBBBBBB
                          ) for
                              BBBBBBBBBB.BBBBBBBB
                      end;
                      AAA =
                          class helper(
                              BBBBBBB
                          ) for
                              BBBBBBBBBB
                                  .BBBBBBBBBB
                      end;
                ",
                record = "
                    type
                      A = record
                      end;
                      AAAAAAAAAAAAAAAAAAAA =
                          record
                      end;
                      AAAA<AAA, AAA, AAA> = record
                      end;
                      AAAAA<AAA, AAA, AAA> =
                          record
                      end;
                      AAAAAAAAAAAAAA<
                          AAA,
                          AAA,
                          AAA
                      > = record
                      end;
                ",
                packed_record = "
                    type
                      A = packed record
                      end;
                      AAAAAAAAAAAAAAAAAAAA =
                          packed record
                      end;
                      AAA<AAAAAAA> = packed record
                      end;
                      AAAA<AAAAAAA> =
                          packed record
                      end;
                      AAA<
                          AAAAAA,
                          AAAAAA,
                          AAAAAA
                      > = packed record
                      end;
                ",
                record_helper = "
                    type
                      A = record helper for BBBBBB
                      end;
                      AAA =
                          record helper for BBBBBB
                      end;
                      AAA =
                          record helper for
                              BBB.BBB
                      end;
                      AAA =
                          record helper for
                              BBBBBBBBBB
                                  .BBBBBBBBBB
                      end;
                ",
                interface = "
                    type
                      A = interface
                      end;
                      AAAAAAAAAAAAAAAAA =
                          interface
                      end;
                      AA = interface(BBBBBBBBBBBB)
                      end;
                      AAA =
                          interface(BBBBBBBBBBBB)
                      end;
                      AAA =
                          interface(
                              BBBBBBBBBBBBBB
                          )
                      end;
                      AA<A, B, C> = interface(BBB)
                      end;
                      AAA<A, B, C> =
                          interface(BBB)
                      end;
                      AAAAAAAAAAAAAAAAAA<
                          A,
                          B,
                          C
                      > = interface(BBBBBBBB)
                      end;
                      AAAAAAAAAAAAAAAAAA<
                          A,
                          B,
                          C
                      > =
                          interface(
                              BBBBBBB.BBBBBB
                          )
                      end;
                ",
                dispinterface = "
                    type
                      A = dispinterface
                      end;
                      AAAAAAAAAAAAA =
                          dispinterface
                      end;
                      AAA<AAAAAAA> = dispinterface
                      end;
                      AAAA<AAAAAAA> =
                          dispinterface
                      end;
                      AAA<
                          AAAAAA,
                          AAAAAA,
                          AAAAAA
                      > = dispinterface
                      end;
                ",
            );
        }
    }

    mod variant_record {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                variant_record = "
                    type
                      TRec = record
                        FField1: TFieldType;
                        FField2: TFieldType;
                      case AAA of
                        BBBBBBBBB: (CCC: DDD);
                        BBBBBBBBBB: (
                          CC: DD;
                          EE: FF
                        );
                        BBBBBBB: (
                          CCCC: DDDDDDD;
                          EEEE: FFFFFFFF
                        );
                        BBBBBBBBBBBBBB: (
                          CCCCCCC:
                              DDDDDDD.DDDDDDD;
                          EEEEEEE:
                              FFFFFFF.FFFFFFFF
                        );
                      end;
                ",
                nested_variant_record = "
                    type
                      TRec = record
                        FField1: TFieldType;
                        FField2: TFieldType;
                      case Boolean of
                        True: (AAA: BBB; CC: DD);
                        False: (
                          AAA: BBB;
                          CC: DD;
                        );
                        False: (
                          AAA: BBB;
                          case Boolean of
                            True: (AAAAAA: BBBBB);
                            False: (
                              AAAAAA: BBBBB;
                              AAAAAAA:
                                  BBBBB.BBBBBB;
                              AAAAAAA:
                                  BBBBBBBBB
                                      .BBBBBB;
                              case Z of
                                A: (
                                  M: N;
                                  case Y of
                                    B: (
                                      O: P;
                                      case X of
                                        C: (
                                          Q: R;
                                          case W of
                                            D: (
                                              S: T;
                                            );
                                        );
                                    );
                                );
                            );
                        );
                      end;
                ",
            );
        }
    }

    mod of_types {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                array_of_const = "
                    type
                      AAAAAAAAAA = array of const;
                      AAAAAAAAAAA =
                          array of const;
                ",
                array_of = "
                    type
                      AAAAAAAAAA = array of AAAAA;
                      AAAAAAAAAAAA =
                          array of AAAAAAAAAAAAAA;
                      AAAAAAAAAAAA =
                          array of
                              AAAAAAAAAAAAAAA;
                      AAAAAAAAAAAA =
                          array of
                              AAAAAAAAAA
                                  .AAAAAAAAA;
                ",
                nested_array_of = "
                    type
                      AAAAAAAAAA = array of const;
                      AAAAAAAAAAA =
                          array of const;
                      AAAAAAAAAAA =
                          array of array of const;
                      AAAAAAAAAAA =
                          array of array of AAAAA;
                      AAAAAAAAAAA =
                          array of
                              array of AAAAAA;
                      AAAAAAAAAAA =
                          array of
                              array of
                                  AAAAAAAAAAA;
                ",
                array_index_of_const = "
                    type
                      AAAA = array[0..1] of const;
                      AAAAA =
                          array[0..1] of const;
                      AAAAA =
                          array[000..11111] of
                              const;
                      AAAAA =
                          array[
                              00000000..1111111111
                          ] of const;
                      AAAAA =
                          array[
                              000000000
                                  ..1111111111
                          ] of const;
                ",
                array_index_of = "
                    type
                      AAAA = array[0..1] of BBBBB;
                      AAAAA =
                          array[0..1] of BBBBB;
                      AAAAA =
                          array[000..11111] of
                              BBBBB;
                      AAAAA =
                          array[000..11111] of
                              BBBBBBBB.BBBBBBB;
                      AAAAA =
                          array[000..11111] of
                              BBBBBBBBBB
                                  .BBBBBBBBB;
                      AAAAA =
                          array[
                              00000000..1111111111
                          ] of BBBBB;
                      AAAAA =
                          array[
                              000000000
                                  ..1111111111
                          ] of BBBBB;
                      AAAAA =
                          array[
                              00000000..1111111111
                          ] of
                              BBBBBBBBB.BBBBBBBBB;
                      AAAAA =
                          array[
                              00000000..1111111111
                          ] of
                              BBBBBBBBB
                                  .BBBBBBBBBB;
                      AAAAA =
                          array[
                              000000000
                                  ..1111111111
                          ] of
                              BBBBBBBBB.BBBBBBBBB;
                      AAAAA =
                          array[
                              000000000
                                  ..1111111111
                          ] of
                              BBBBBBBBB
                                  .BBBBBBBBBB;
                ",
                set_of = "
                    type
                      AAAAAAAAAAAAA = set of BBBB;
                      AAAAAAAAAAAA = set of BB.BB;
                      AAAAAAAAAAAAAA =
                          set of BBBB;
                      AAAAAAAAAAAAA =
                          set of BB.BB;
                      AAAAAAAAAAAAAA =
                          set of
                              BBBBBBBBBBBBBBBBBBB;
                      AAAAAAAAAAAAA =
                          set of
                              BBBBBBBBBB.BBBBBBBB;
                      AAAAAAAAAAAAA =
                          set of
                              BBBBBBBBBB
                                  .BBBBBBBBBBB;
                      AAAAAAAAAAAAA = set of (AA);
                      AAAAAAAAAAAAA =
                          set of (AAA);
                      AAAAAAAAAAAAA =
                          set of (AAAA, BBB, CCC);
                      AAAAAAAAAAAAA =
                          set of
                              (AAAA, BBBB, CCC);
                      AAAAAAAAAAAAA =
                          set of
                              (
                                  AAAAA,
                                  BBBBB,
                                  CCCC
                              );
                ",
                file = "
                    type
                      AAAAAAAAAAAAAAAAAAAA = file;
                      AAAAAAAAAAAAAAAAAAAAA =
                          file;
                ",
                file_of = "
                    type
                      AAAAAAA = file of BBBBBBBBB;
                      AAAAAAA = file of BBBB.BBBB;
                      AAAAAAAA =
                          file of BBBBBBBBB;
                      AAAAAAAA =
                          file of BBBB.BBBB;
                      AAAAAAAA =
                          file of
                              BBBBBBBBBBBBBBBBBB;
                      AAAAAAAA =
                          file of
                              BBBBBBBBB.BBBBBBBB;
                      AAAAAAAA =
                          file of
                              BBBBBBBBB
                                  .BBBBBBBBBBBB;
                ",
                class_of = "
                    type
                      AAAAAAAA = class of BBBBBBB;
                      AAAAAAAA = class of BBB.BBB;
                      AAAAAAAAA =
                          class of BBBBBBB;
                      AAAAAAAAA =
                          class of BBB.BBB;
                      AAAAAAAAA =
                          class of
                              BBBBBBBBBBBBBBBBB;
                      AAAAAAAAA =
                          class of
                              BBBBBBBB.BBBBBBBB;
                      AAAAAAAAA =
                          class of
                              BBBBBBBBBBB
                                  .BBBBBBBBBB;
                ",
                type_of = "
                    type
                      AAAAAAAAA = type of BBBBBBB;
                      AAAAAAAAA = type of BBB.BBB;
                      AAAAAAAAAA =
                          type of BBBBBBB;
                      AAAAAAAAAA =
                          type of BBB.BBB;
                      AAAAAAAAAA =
                          type of
                              BBBBBBBBBBBBBBBBBB;
                      AAAAAAAAAA =
                          type of
                              BBBBBBBB.BBBBBBBBB;
                      AAAAAAAAAA =
                          type of
                              BBBBBBBBBBB
                                  .BBBBBBBBBB;
                ",
            );
        }
    }

    mod procedural {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                procedure_of_object = "
                    type
                      A = procedure of object;
                      AAAAAA =
                          procedure of object;
                      AAAAAA =
                          procedure(AA: B) of
                              object;
                      AAAAAA =
                          procedure(
                              AA: BB;
                              CC: DD
                          ) of object;
                ",
                procedure_reference = "
                    type
                      AA = reference to procedure;
                      AAA =
                          reference to procedure;
                      AAA =
                          reference to
                              procedure(AAA: BBB);
                      AAA =
                          reference to
                              procedure(
                                  AAAA: BBB
                              );
                      AAA =
                          reference to
                              procedure(
                                  AA: B;
                                  CC: D
                              );
                ",
                simple_procedure = "
                    type
                      AA = procedure(AAAA: BBBBB);
                      AAA =
                          procedure(
                              AAAAA: BBBBBB
                          );
                      AAA =
                          procedure(AA: BB; C: D);
                      AAA =
                          procedure(
                              AAA: BB;
                              CCC: DD
                          );
                ",
                function_of_object = "
                    type
                      A = function: AAA of object;
                      AAAAAA =
                          function: AAA of object;
                      AAAAAA =
                          function(AA: B):
                              AAAA of object;
                      AAAAAA =
                          function(
                              AA: BB;
                              CC: DD
                          ): AAAAAA of object;
                ",
                function_reference = "
                    type
                      AAA = reference to function;
                      AAA =
                          reference to
                              function: BBBBBBBBB;
                      AAA =
                          reference to
                              function(
                                  AA: B;
                                  CC: D
                              ): BBBBBBB;
                ",
                simple_function = "
                    type
                      AA = function(AA: BB): CCCC;
                      AAA =
                          function(AA: BB): CCCC;
                      AAA =
                          function(
                              AAA: BB;
                              CCC: DD
                          ): EEEEE;
                ",
            );
        }
    }

    mod string {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                string = "
                    type
                      AAAAAAAAAAAAAAAAAA = string;
                      AAAAAAAAAAAAAAAAAAA =
                          string;
                ",
                short_string_literal = "
                    type
                      AAAAAAAAAAAAA = string[255];
                      AAAAAAAAAAAAA =
                          string[BBBB];
                      AAAAAAAAAAAAA =
                          string[
                              BBBBBBBB + CCCCCCC
                          ];
                      AAAAAAAAAAAAA =
                          string[
                              BBBBBBBBBBB
                                  + CCCCCCCCC
                          ];
                ",
            );
        }
    }

    mod ordinal {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                sub_range = "
                    type
                      AAAAAAAAAA = BBBBBB..CCCCCC;
                      AAAAAAAAAAA =
                          BBBBBB..CCCCCC;
                      AAAAAAAAAAA =
                          BBBBBBBBBBBB
                              ..CCCCCCCCCCCC;
                ",
                enum_type = "
                    type
                      AAAAAA = (BBBB, CCCC, DDDD);
                      AAAAAAA =
                          (BBBB, CCCC, DDDD);
                      AAAAAAA = (
                          BBBBBB,
                          CCCCCC,
                          DDDDDD
                      );
                ",
                enum_with_val = "
                    type
                      AAAAAA = (BBB = 1, CCC = 2);
                      AAAAAAA =
                          (BBB = 1, CCC = 2);
                      AAAAAAA = (
                          BBB = 1,
                          CCC = 2,
                          DDDD
                      );
                ",
            );
        }
    }

    mod aliases {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                strong_alias = "
                    type
                      AAAAAAAAA = type BBBBBBBBBB;
                      AAAAAAAAA = type BBBBB.BBBB;
                      AAAAAAAAAA =
                          type BBBBBBBBBB;
                      AAAAAAAAAA =
                          type BBBBB.BBBB;
                      AAAAAAAAAA =
                          type
                              BBBBBBBBBBBBBBBBBBB;
                      AAAAAAAAAA =
                          type
                              BBBBBBBBBBBBBBBBBBBBB;
                      AAAAAAAAAA =
                          type
                              BBBBBBBBB.BBBBBBBBB;
                      AAAAAAAAAA =
                          type
                              BBBBBBBBB
                                  .BBBBBBBBBB;
                ",
                weak_alias = "
                    type
                      AAAAAAAAAAAA = BBBBBBBBBBBB;
                      AAAAAAAAAAAA = BBBBBB.BBBBB;
                      AAAAAAAAAAAAA =
                          BBBBBBBBBBBB;
                      AAAAAAAAAAAAA =
                          BBBBBB.BBBBB;
                      AAAAAAAAAAAAA =
                          BBBBBBBBBBB
                              .BBBBBBBBBBBBBB;
                ",
                pointer = "
                    type
                      AAAAAAAAAAAAA = ^BBBBBBBBBB;
                      AAAAAAAAAAAAA =
                          ^BBBBBBBBBBB;
                      AAAAAAAAAAAAA =
                          ^BBBBBBBBBBBBBBBBBBBBBBB;
                ",
            );
        }
    }

    mod fields {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                class = "
                    type
                      A = class
                        AAAAAAA: BBBBBBBBBBBBBBBB;
                        AAAAAAAA:
                            BBBBBBBBBBBBBBBB;
                      end;
                ",
            );
        }
    }
}

mod prop_decls {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            no_directives = "
                    property Appless: Bananasssss;
                    property Applesss:
                        Bananasssss;
                    property Applessssssssssssssss:
                        Bananasssss;
            ",
            one_directive = "
                    property AAAA: BBBB read CCCC;
                    property AAAA: BBBB
                        read CCCCC;
            ",
            many_directives = "
                    property A: B read C write D;
                    property A: B
                        read CC write DD index EE;
                    property A: B
                        read CCC
                        write DD
                        index EEEE;
            ",
            directive_expression = "
                    property A: B index CCC + DDD;
                    property AA: BB
                        index CCC + DDD;
                    property AA: BB
                        index
                            CCCCCCCCCCC
                                + DDDDDDDDD
                        read EEE;
                    property AA: BB
                        index
                            CCCCCCCCCCCCCCCCCCCC
                                + DDDDDDDDD
                                + EEEEEEEEE
                        read FFF;
            ",
            directive_without_expression = "
                    property A: B nodefault;
                    property A: B
                        nodefault default;
                    property A: B
                        nodefault
                        nodefault
                        default;
            ",
        );
    }
}

mod const_records {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            array_of = "
                    const
                      C_AA: array[0..1] of AAA = (
                          (
                              AAAAA: BBBBB;
                              CCCC: DDDD
                          ),
                          (EEE: FFF; GGG: HHH),
                          (III: JJJ; KKK: LLL),
                          (MMM: NNN; OOO: PPP)
                      );
                      C_AA: array[0..1] of AAA = (
                          (AAA: BBB; CCC: DDD),
                          (EEE: FFF; GGG: HHH),
                          (
                              IIIII: JJJJJ;
                              KKKK: LLLL
                          ),
                          (MMM: NNN; OOO: PPP)
                      );
                      C_AA: array[0..1] of AAA = (
                          (AA: BB; CC: DD; E: FF),
                          (
                              GGGGGGGGG: HHHHHHHH;
                              GGGGGGGGG:
                                  HHHHHHHHHHH;
                              IIIIIII:
                                  JJJJJJJJ(
                                      JJJ,
                                      JJJ
                                  );
                              IIIIIII:
                                  JJJJJJJ(JJ, JJ);
                              JJJJJJJJJ:
                                  (JJJ, JJJ, JJJ);
                              JJJJJJJJJ: (
                                  JJJJ,
                                  JJJ,
                                  JJJ
                              );
                              JJJJJJJJJ: (
                                  JJJJ + JJJ + JJJ
                              );
                              JJJJJJJJJ: (
                                  JJJJ
                                      + JJJJ
                                      + JJJ
                              );
                              JJJJJJJJJ: (
                                      JJJJ
                                          + JJJJ
                                          + JJJ)
                                  + JJJ;
                              KK: LL
                          ),
                          (MM: NN; OO: PP; QQ: RR)
                      );
                      C_AAAAA:
                              array[0..1] of AAA =
                          (
                              (AAA: BBB; CCC: DD),
                              (EEE: FFF; GGG: HHH)
                          );
                      C_AAAAA:
                              array[
                                  00000..1111
                              ] of AA =
                          (
                              (AAA: BBB; CCC: DD),
                              (EEE: FFF; GGG: HHH)
                          );
                      C_AAAAA:
                              array[0..1] of
                                  AAAA =
                          (
                              (AAA: BBB; CCC: DD),
                              (EEE: FFF; GGG: HHH)
                          );
                ",
            set_of = "
                    const
                      C_AA: set of AAA = (
                          (
                              AAAAA: BBBBB;
                              CCCC: DDDD
                          ),
                          (EEE: FFF; GGG: HHH),
                          (III: JJJ; KKK: LLL),
                          (MMM: NNN; OOO: PPP)
                      );
                      C_AA: set of AAA = (
                          (AAA: BBB; CCC: DDD),
                          (EEE: FFF; GGG: HHH),
                          (
                              IIIII: JJJJJ;
                              KKKK: LLLL
                          ),
                          (MMM: NNN; OOO: PPP)
                      );
                      C_AA: set of AAA = (
                          (AA: BB; CC: DD; E: FF),
                          (
                              GGGGGGGGG: HHHHHHHH;
                              GGGGGGGGG:
                                  HHHHHHHHHHH;
                              IIIIIII:
                                  JJJJJJJJ(
                                      JJJ,
                                      JJJ
                                  );
                              IIIIIII:
                                  JJJJJJJ(JJ, JJ);
                              KK: LL
                          ),
                          (MM: NN; OO: PP; QQ: RR)
                      );
                ",
            single = "
                    const
                      C_AA: AAA = (A: B; C: D);
                      C_AAAAAA: AAA =
                          (A: B; C: D);
                      C_AAAAAAAAAA:
                              AAAAAAAAAAAAA =
                          (A: B; C: D);
                      C_AAAAAA: AAA = (
                          AAAA: BBBB;
                          CCCC: DDDD
                      );
                      C_AAAAAA: AAA = (
                          AAAAAAAAAAA:
                              BBBBBBBBBBB;
                          CCCC: DDDD
                      );
                      C_AAAAAA: AAA = (
                          AAAAAAAA: BBBBBBBB;
                          CCCCCCCCCCC:
                              DDDDDDDDDDDD
                      );
                      C_AAAAAAAAAAAA:
                              AAAAAAAAAAA =
                          (
                              AAAAAAAA: BBBBBBBB;
                              CCCCCCCCCC:
                                  DDDDDDDDD
                          );
                ",
        );
    }
}

mod routines {
    use super::*;

    pub fn generate(root_dir: &Path) {
        params::generate(root_dir);
        directives::generate(root_dir);
        decl_sections::generate(root_dir);
    }

    mod params {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                no_params_procedure = "
                    interface
                    procedure Applessssssssssssss;
                    procedure Applesssssssssssssss;
                ",
                no_params_function = "
                    interface
                    function Applessss: Bananasss;
                    function Applessssssssssssssss:
                        Booo;
                    function Applessss:
                        Bananassss;
                ",
                empty_param_list_procedure = "
                    interface
                    procedure Applessssssssssss();
                    procedure Applesssssssssssss();
                    procedure Applessssssssssssss();
                    procedure Applesssssssssssssss();
                ",
                empty_param_list_function = "
                    interface
                    function Appless(): Bananasss;
                    function Applessssssssssssss():
                        Booo;
                ",
                one_param_procedure = "
                    interface
                    procedure Apple(Boo: Carrots);
                    procedure Apples(
                        Boo: Carrots
                    );
                    procedure Apples(
                        Boo:
                            Carrotssssssssssssssssss
                    );
                    procedure Apples(
                        Boooooo,
                        Carrotss: Durians
                    );
                ",
                one_param_function = "
                    interface
                    function AA(Booo: Carrots): D;
                    function AAA(Booo: Carrots):
                        D;
                    function AAA(
                        Boooooo: Carrots
                    ): D;
                    function AA(
                        Boo:
                            Carrotssssssssssssssss
                    ): D;
                    function Apples(
                        Boo:
                            Carrotssssssssssssssss
                    ): D;
                ",
                const_param = "
                    interface
                    procedure A(const AAAA: AAAA);
                    procedure AA(
                        const AAAA: AAAA
                    );
                    procedure AA(
                        const AAAA: AAAA;
                        const BBBB: BBBB
                    );
                    procedure AA(
                        const AAAAAAAAAA:
                            AAAAAAAAA
                    );
                    procedure AA(
                        const AAAAAAAAAA:
                            AAAAAAAAA;
                        const BBBBBBBBBB:
                            BBBBBBBBB
                    );
                    procedure AA(
                        const AAAAAAAAAAAAAAAAAAAA:
                            AAAAAAAAA
                    );
                    procedure AA(
                        const AAAAAAAAAAAAAAAAAAAA:
                            AAAAAAAAA;
                        const BBBBBBBBBBBBBBBBBBBB:
                            BBBBBBBBB
                    );
                ",
                var_param = "
                    interface
                    procedure A(var AAAA: AAAA);
                    procedure AAAA(
                        var AAAA: AAAA
                    );
                    procedure AA(
                        var AAAA: AAAA;
                        var BBBB: BBBB
                    );
                    procedure AA(
                        var AAAAAAAAAAAA:
                            AAAAAAAAA
                    );
                    procedure AA(
                        var AAAAAAAAAAAA:
                            AAAAAAAAA;
                        var BBBBBBBBBBBB:
                            BBBBBBBBB
                    );
                    procedure AA(
                        var AAAAAAAAAAAAAAAAAAAAAA:
                            AAAAAAAAA
                    );
                    procedure AA(
                        var AAAAAAAAAAAAAAAAAAAAAA:
                            AAAAAAAAA;
                        var BBBBBBBBBBBBBBBBBBBBBB:
                            BBBBBBBBB
                    );
                ",
                comma_param_function = "
                    interface
                    function Apples(
                        Boooooo,
                        Carrotss: Durians
                    ): D;
                    function Apples(
                        Booooooooo,
                        Carrotssssssss:
                            Duriansssss
                    ): D;
                    function Apples(
                        Boooooooo,
                        Carrotss,
                        Durianss: Eggplants
                    ): D;
                    function Apples(
                        Boooooooo,
                        Carrotss,
                        Duriansssss:
                            Eggplantssssss
                    ): D;
                ",
                many_param_procedure = "
                    interface
                    procedure Appless(B: C; D: E);
                    procedure Applesss(
                        B: C;
                        D: E
                    );
                    procedure Apples(
                        Boo:
                            Carrotsssssssssssssss;
                        Dee:
                            Eggplantsssssssssssss;
                    );
                    procedure Apples(
                        Bananas, Carrots: Durians;
                        Eggplantss, Figsss: Grapes
                    );
                    procedure Apples(
                        Bananasss,
                        Carrotss,
                        Durianss: Eggplantss;
                        Figsssss,
                        Grapess,
                        Habaneros: Iceberg
                    );
                ",
                qualified_name = "
                    interface
                    procedure Apples.Banana(A: B);
                    procedure Apple.Boo.Caa(A: B);
                    procedure Apples.Bananas(
                        AAAAAAAAAAAAA: B
                    );
                    procedure Apple.Boo.Caaa(
                        A: B
                    );
                    procedure Applessss.Bananassss(
                        AAAAAAAAAA: BBBBBBBBBBBB =
                            CCCCCCCCCCC
                                .DDDDDDDDDDD
                    );
                    procedure Apple.Booooo.Caaaaaa(
                        A: B
                    );
                    function Apple.Booooo.Caaaaaaa(
                        AAAAAAAAAA:
                            BBBBBBBBBBBBB
                                .BBBBBBBBB
                    ):
                        AAAAAAAAAAAAA
                            .AAAAAAAAAAAAA;
                ",
                generic_params = "
                    interface
                    procedure A(B: C<T>; D: E<V>);
                    procedure AA(
                        B: C<T>;
                        D: E<V>
                    );
                    procedure Apples(
                        BBBBBBBBB:
                            CCCCCCCCCCCC<T, T, T>;
                        DDDDDDDDD:
                            EEEEEEEEEEEEE<V, V, V>
                    );
                    procedure Apples(
                        BBBBBBBBB:
                            CCCCCCCCCCCCC<
                                T,
                                T,
                                T
                            >;
                        DDDDDDDDD:
                            EEEEEEEEEEEEEE<
                                V,
                                V,
                                V
                            >
                    );
                ",
                generic_routines = "
                    interface
                    procedure Apples<A; B; C>();
                    procedure Apples<
                        AA;
                        BB;
                        CC
                    >();
                    procedure Apples<
                        AA: record;
                        BB: constructor;
                        CC
                    >();
                    procedure Apples<
                        AA, BB: record;
                        AAAAAAAAAA, BBBBBBBB:
                            IInterface;
                        AAAAAAAA,
                        BBBBBBBB,
                        CCCCCCC: record;
                        CCCCCCCCCCCCC:
                            constructor;
                        CCCCCCCCCCCCC:
                            constructor, record;
                        CC:
                            constructor,
                            record,
                            class;
                        CC:
                            constructor,
                            record,
                            class,
                            IInterface
                    >();
                ",
                generic_routines_and_params = "
                    interface
                    procedure Apples<A; B>(CC: D);
                    procedure Apples<A; B>(
                        CC: DD;
                        EE: FF
                    );
                    procedure Apples<
                        AAA;
                        BBB;
                        CC
                    >(CC: DD; EE: FF);
                    procedure Apples<
                        AAA;
                        BBB;
                        CC
                    >(
                        CCCCCC: DDDDD;
                        EEEEE: FFFFF
                    );
                ",
                single_default = "
                    interface
                    procedure Applessss(A: B = 1);
                    procedure Applesssss(
                        A: B = 1
                    );
                    procedure Applesssss(
                        AAAAAAAAAAAA: BBBBBBBBB =
                            1
                    );
                    procedure Applesssss(
                        AAAAAAAAAAAA:
                                BBBBBBBBBBB =
                            1
                    );
                ",
                many_defaults = "
                    interface
                    function Applesss(
                        A: B = 1;
                        C: D = 1
                    ): C;
                    function Applesss(
                        AAAAAAAAAAAAAA: BBBBBBB =
                            1;
                        C: D = 1
                    ): C;
                    function Applesss(
                        AAAAAAAAAAAAAA:
                                BBBBBBBBB =
                            1;
                        C: D = 1
                    ): C;
                    function Applesss(
                        A: B = 1;
                        CCCCCCCCCCC: DDDDDDDDDD =
                            1
                    ): C;
                    function Applesss(
                        A: B = 1;
                        CCCCCCCCCCCC:
                                DDDDDDDDDDD =
                            1
                    ): C;

                ",
                default_expression = "
                    interface
                    procedure A(
                        A: B =
                            111111 + 1111 + 1111
                    );
                    procedure A(
                        A: B =
                            111111
                                + 111111
                                + 11111
                    );
                    procedure A(
                        A: B =
                            AAAAA(
                                    11111,
                                    22222,
                                    33)
                                + 111111
                                + 11111
                    );
                    procedure A(
                        AAAAAAAAAAAAA:
                                BBBBBBBBBB =
                            11111 + 1111 + 111
                    );
                    procedure A(
                        AAAAAAAAAAAAA:
                                BBBBBBBBBB =
                            111111
                                + 111111
                                + 11111
                    );
                    procedure A(
                        AAAAAAAAAAAAA:
                                BBBBBBBBBB =
                            AAAAA(
                                    11111,
                                    22222,
                                    33)
                                + 111111
                                + 11111
                    );
                ",
                constructor = "
                    interface
                    constructor AAAAAAAA(AAA: AA);
                    constructor AAAAAAAA(
                        AAA: AAA
                    );
                    constructor AAAAAAAA(
                        AAA: AAA;
                        BBB: BBB
                    );
                ",
                destructor = "
                    interface
                    destructor AAAAAAAA(AAA: AAA);
                    destructor AAAAAAAAA(
                        AAA: AAA
                    );
                    destructor AAAAAAAA(
                        AAA: AAA;
                        BBB: BBB
                    );
                ",
            );
        }
    }

    mod directives {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                single = "
                    interface
                    procedure Applessss; overload;
                    procedure Applesssss;
                        overload;
                    function Appless: B; overload;
                    function Applesss: B;
                        overload;
                ",
                many = "
                    interface
                    procedure A; overload; static;
                    procedure AA;
                        overload; static; static;
                    procedure AA;
                        overload;
                        static;
                        deprecated;
                ",
                with_params = "
                    interface
                    function A(B: C): D; overload;
                    function AA(B: C): D;
                        overload;
                    function AA(B: C): D;
                        overload; static; static;
                    function AA(B: C): D;
                        overload;
                        static;
                        deprecated;
                ",
                with_args = "
                    interface
                    procedure A(B: C); message 11;
                    procedure A(B: C);
                        message 111;
                    procedure A(B: C);
                        message 1111 + 1111111;
                    procedure A(B: C);
                        message
                            1111 + 11111 + 1111;
                        message
                            111111
                                + 111111
                                + 1111;
                        message
                            11111
                                + fooo(
                                    aaaaaa,
                                    bbbbbbb)
                                + 1111;
                    procedure A(B: C);
                        message 11; deprecated '';
                    procedure A(B: C);
                        message 11;
                        deprecated '';
                        static;
                ",
            );
        }
    }

    mod decl_sections {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                var = "
                    procedure Foo;
                    var
                      AAAAAAA: BBBBBBBBBBBBBBBBBB;
                      AAAAAAAA:
                          BBBBBBBBBBBBBBBBBB;
                    var
                      AAAAAAAA:
                          BBBBBBBBBBBBBBBBBB;
                      AAAAAAA: BBBBBBBBBBBBBBBBBB;
                    begin
                    end;
                ",
                label = "
                    procedure Foo;
                    label
                      AAAAAAAAAA, BBBBBBBBBBB, CC;
                    label
                      AAAAAAAAAA,
                      BBBBBBBBBBB,
                      CCC;
                    begin
                    end;
                ",
            );
        }
    }
}

mod control_flows {
    use super::*;

    pub fn generate(root_dir: &Path) {
        if_then::generate(root_dir);
        else_if_then::generate(root_dir);
        while_do::generate(root_dir);
        with_do::generate(root_dir);
        for_each::generate(root_dir);
        for_in::generate(root_dir);
        case::generate(root_dir);
        except_on::generate(root_dir);
        repeat_until::generate(root_dir);
    }

    mod if_then {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                inline = "
                    if AAAAAAAA and BBBBBBBBB then
                      Bar;
                    if AAAAAAAA
                        and BBBBBBBBBB then
                      Bar;
                    if AAAAAAAAAA
                        and BBBBBBBBBB
                        and CCCCCCCCCC then
                      Bar;
                    if AAA(AAAA, BBBBBB, CCC) then
                      Bar;
                    if AAA(
                        AAAA,
                        BBBBBB,
                        CCCC) then
                      Bar;
                ",
                compound = "
                    if AAAAAA and BBBBB then begin
                      Bar;
                    end;
                    if AAAAAA and BBBBBB then
                    begin
                      Bar;
                    end;
                    if AAAAAAAAAA
                        and BBBBBBBB then
                    begin
                      Bar;
                    end;
                    if AAAAAAAAAA
                        and BBBBBBBBBB
                        and CCCCCCCCCC then
                    begin
                      Bar;
                    end;
                    if AAA(AA, BB, CCC) then begin
                      Bar;
                    end;
                    if AAA(AAA, BB, CCC) then
                    begin
                      Bar;
                    end;
                    if AAA(
                        AAAA,
                        BBBBBB,
                        CCCC) then
                    begin
                      Bar;
                    end;
                ",
            );
        }
    }

    mod else_if_then {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                inline = "
                    if A then
                    else if AAAAAA and BBBBBB then
                      Bar;
                    else if AAAAAAAAA
                        and BBBBBBBBB then
                      Bar;
                    else if AAAAAAAAAA
                        and BBBBBBBBBB
                        and CCCCCCCCCC then
                      Bar;
                    else if Foo(AAA, BB, CCC) then
                      Bar;
                    else if Foo(
                        AAAA,
                        BBBBBB,
                        CCCC) then
                      Bar;
                ",
                compound = "
                    if A then begin
                    end
                    else if AAA and BBB then begin
                      Bar;
                    end
                    else if AAAAAA and BBBBBB then
                    begin
                      Bar;
                    end
                    else if AAAAAAAAAAA
                        and BBBBBBBBB then
                    begin
                      Bar;
                    end
                    else if AAAAAAAAAA
                        and BBBBBBBBBB
                        and CCCCCCCCCC then
                    begin
                      Bar;
                    end
                    else if AA(A, B, C) then begin
                      Bar;
                    end;
                    else if AAA(AAA, BB, CCC) then
                    begin
                      Bar;
                    end
                    else if AAA(
                        AAAA,
                        BBBBB,
                        CCCCC) then
                    begin
                      Bar;
                    end
                ",
            );
        }
    }

    mod while_do {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                inline = "
                    while AAAAAAAAA = BBBBBBBBB do
                      Bar;
                    while AAAAAAAAAAA
                        = BBBBBBBBBB do
                      Bar;
                    while AAAAAAAAAA
                        = BBBBBBBBBB
                            + CCCCCCCCCC do
                      Bar;
                    while AAA(AAAA, BBBB, CCCC) do
                      Bar;
                    while AAAAAA(
                        AAAA,
                        BBBB,
                        CCCC) do
                      Bar;

                    while not not A and B and C do
                      ;
                    while not not AA
                        and BB
                        and CC do
                      ;
                ",
                compound = "
                    while AAAAAA = BBBBBB do begin
                    end;
                    while AAAAAAA = BBBBBB do
                    begin
                    end;
                    while AAAAAAAAAAA
                        = BBBBBBBBBB do
                    begin
                    end;
                    while AAAAAAAAAA
                        = BBBBBBBBBB
                            + CCCCCCCCCC do
                    begin
                    end;
                    while AA(AA, BB, CCC) do begin
                    end;
                    while AAAAAA(AAA, BBB, CCC) do
                    begin
                    end;
                    while AAAAAAAAA(
                        AAA,
                        BBB,
                        CCC) do
                    begin
                    end;
                ",
            );
        }
    }

    mod with_do {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                inline = "
                    with AAAAAAA(BBBBBBB, CCCC) do
                      Bar;
                    with AAAAAAA(
                        BBBBBBB,
                        CCCCC) do
                      Bar;
                    with AAAAAA, BBBBBB, CCCCCC do
                      Bar;
                    with
                        AAAAAA, BBBBBBB, CCCCCC do
                      Bar;
                    with
                        AAAAAAA,
                        BBBBBBB,
                        CCCCCC do
                      Bar;
                    with AAA(A), BBB(B), CCC(C) do
                      Bar;
                    with
                        AAA(AA), BBB(B), CCC(C) do
                      Bar;
                    with
                        AAA(AA),
                        BBB(BB),
                        CCC(C) do
                      Bar;
                    with
                        AAAA(
                            AAAAAAA,
                            AAAAAAAAAAA),
                        BBBBBBBBBBB(BBBBBB, BBBB),
                        CCCCCCCCCCCC(CCCC, CCC) do
                      Bar;
                    with
                        AAA(AAAAAAA, AAAAAAAAAA),
                        BBBBBBBBBBB(
                            BBBBBB,
                            BBBBB),
                        CCCCCCCCCCCC(CCCC, CCC) do
                      Bar;
                    with
                        AAA(AAAAAAA, AAAAAAAAAA),
                        BBBBBBBBBBB(BBBBBB, BBBB),
                        CCCCCCCCCCCC(
                            CCCC,
                            CCCC) do
                      Bar;
                ",
                compound = "
                    with AAAA(BBBB, CCCC) do begin
                    end;
                    with AAAAA(BBBB, CCCC) do
                    begin
                    end;
                    with AAAAAAA(
                        BBBBBBB,
                        CCCCCC) do
                    begin
                    end;
                    with AAAA, BBBB, CCCC do begin
                    end;
                    with AAAAA, BBBB, CCCC do
                    begin
                    end;
                    with
                        AAAAAA, BBBBBBB, CCCCCC do
                    begin
                    end;
                    with
                        AAAAAAA,
                        BBBBBBB,
                        CCCCCC do
                    begin
                    end;
                    with A(A), B(B), C(C) do begin
                    end;
                    with AA(A), B(B), C(C) do
                    begin
                    end;
                    with
                        AAA(AA), BBB(B), CCC(C) do
                    begin
                    end;
                    with
                        AAA(AA),
                        BBB(BB),
                        CCC(C) do
                    begin
                    end;
                    with
                        AAAA(
                            AAAAAAA,
                            AAAAAAAAAAA),
                        BBBBBBBBBBB(BBBBBB, BBBB),
                        CCCCCCCCCCCC(CCCC, CCC) do
                    begin
                    end;
                    with
                        AAA(AAAAAAA, AAAAAAAAAA),
                        BBBBBBBBBBB(
                            BBBBBB,
                            BBBBB),
                        CCCCCCCCCCCC(CCCC, CCC) do
                    begin
                    end;
                    with
                        AAA(AAAAAAA, AAAAAAAAAA),
                        BBBBBBBBBBB(BBBBBB, BBBB),
                        CCCCCCCCCCCC(
                            CCCC,
                            CCCC) do
                    begin
                    end;
                ",
            );
        }
    }

    mod for_each {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                predefined_var_to = "
                    for AAAAA := BBBBB to CCCCC do
                      A;
                    for AAAAAA := BBBBB
                        to CCCCC do
                      A;
                    for AAAAAAAAAA := BBBBB(0)
                        to CCCCC do
                      A;
                    for AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCC do
                      A;
                    for AAAAAAAAAA := BBBBBBBB
                        to CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for AAAAAAAAAA := BBBBBBBB
                        to CCCCCCCC.DDDDDDDD
                            + 1 do
                      A;
                    for AAAAAAAAAA := BBBBBBBB
                        to CCCCCCCCCCCC
                                .DDDDDDDDDDD
                            + 1 do
                      A;
                    for AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for AAAAAAAAAAAAAAAAAAAA :=
                            BBBBBBBBB
                        to CCCCC do
                      A;
                ",
                predefined_var_downto = "
                    for AAAAA := BBB downto CCC do
                      A;
                    for AAAAAA := BBB
                        downto CCC do
                      A;
                    for AAAAAAAAAA := BBBBB(0)
                        downto CCCCC do
                      A;
                    for AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        downto CCCCC do
                      A;
                    for AAAAAAAAAA := BBBBBBBB
                        downto CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        downto CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for AAAAAAAAAAAAAAAAAAAA :=
                            BBBBBBBBB
                        downto CCCCC do
                      A;
                ",
                untyped_var = "
                    for var AAA := BBB to CCCCC do
                      A;
                    for var AAAAAA := BBBBB
                        to CCCCC do
                      A;
                    for var AAAAAAAAAA := BBBBB(0)
                        to CCCCC do
                      A;
                    for var AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCC do
                      A;
                    for var AAAAAAAAAA := BBBBBBBB
                        to CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for var AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for var AAAAAAAAAAAAAAAAAAAA :=
                            BBBBBBBBB
                        to CCCCC do
                      A;
                ",
                typed_var = "
                    for var AAA: AA := BB to CC do
                      A;
                    for var AAAAAA: AAA := BBBBB
                        to CCCCC do
                      A;
                    for var AAAAAA: AA := BBBBB(0)
                        to CCCCC do
                      A;
                    for var AAAAAAAAAA: AA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCC do
                      A;
                    for var AAAAAAAAAA:
                                AAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCC do
                      A;
                    for var AAAAAAAA:
                                AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCCCCCC(
                            0000,
                            1111) do
                      A;
                    for var AAAAAAAAAA:
                                AAAAAAAA :=
                            BBBBBBBBB
                        to CCCCC do
                      A;
                ",
                for_each_compound = "
                    for AAA := BBB to CCC do begin
                    end;
                    for AAAAA := BBBBB to CCCCC do
                    begin
                    end;
                    for AAAAAA := BBBBB
                        to CCCCC do
                    begin
                    end;
                    for var AAAAAAAAAA := BBBBB(0)
                        to CCCCC do
                    begin
                    end;
                    for var AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCC do
                    begin
                    end;
                    for var AAAAAAAAAA := BBBBBBBB
                        to CCCCCCCCC(
                            0000,
                            1111) do
                    begin
                    end;
                    for var AAAAAAAAAA :=
                            BBBBBBBB(
                                000000,
                                11111)
                        to CCCCCCCCC(
                            0000,
                            1111) do
                    begin
                    end;
                    for var AAAAAAAAAAAAAAAAAAAA :=
                            BBBBBBBBB
                        to CCCCC do
                    begin
                    end;
                ",
            );
        }
    }

    mod for_in {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                predefined_var = "
                    for AAAAAAAAAA in BBBBBBBBB do
                      A;
                    for AAAAAAAAAA
                        in BBBBBB(00000, 11111) do
                      A;
                    for AAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                      A;
                    for AAAAAAAAAAAAAAAAAAAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                      A;
                ",
                untyped_var = "
                    for var AAAAAAAA in BBBBBBB do
                      A;
                    for var AAAAAAAAAA
                        in BBBBBB(00000, 11111) do
                      A;
                    for var AAAAAAAAAAAAAAAAAAAAAAA
                        in BBBBBB(00000, 11111) do
                      A;
                    for var AAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                      A;
                    for var AAAAAAAAAAAAAAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                      A;
                ",
                typed_var = "
                    for var AAA: AAA in BBBBBBB do
                      A;
                    for var AAAAAAAAAA: AAAAA
                        in BBBBBB(00000, 11111) do
                      A;
                    for var AAAAAAAAAA:
                            AAAAAAAAAAA
                        in BBBBBB(00000, 11111) do
                      A;
                    for var AAAAAAAAAA:
                            AAAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                      A;
                ",
                for_in_compound = "
                    for AAAAAAA in BBBBBB do begin
                    end;
                    for AAAAAAAAAA in BBBBBBBBB do
                    begin
                    end;
                    for AAAAAAAAAA
                        in BBBBBB(00000, 11111) do
                    begin
                    end;
                    for AAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                    begin
                    end;
                    for AAAAAAAAAAAAAAAAAAAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                    begin
                    end;
                    for var AAAAAAAAAAAAAAAAAAAAAAA
                        in BBBBBBB(
                            00000,
                            11111) do
                    begin
                    end;
                ",
                many_ins = "
                    for aaaaaaaaaa
                        in [
                            aa in [aa, bbb],
                            bbbb] do
                "
            );
        }
    }

    mod case {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                header = "
                    case AAAAA.BBBBBB(CCC, DDD) of
                    end;
                    case AAAAAA
                        .BBBBBBB(CCC, DDD) of
                    end;
                    case AAAAA.BBBBBBB(
                        CCCCCC,
                        DDDDDD) of
                    end;
                    case AAAAA
                        .BBBBBBBB(
                            CCCCCCC,
                            DDDDDDD)
                        .EEEEEEEE(FFFFFFFF) of
                    end;
                ",
                case_arm = "
                    case A of
                      AAAAAAAAAAAAAA:;
                      AAAAAAAAAAAA, BBBBBBBBBB:;
                      AAAAAAAA, BBBBBBB, CCCCCCC:;
                      AAAAAAAAAAAAA,
                      BBBBBBBBBBBBB:;
                      AAAAAAAAA,
                      BBBBBBBB,
                      CCCCCCC:;
                      AAAA..BBBB, CCC..DDD:;
                      A..B, C..D, E..F:;
                      AAAAAA..BBBBBB,
                      CCCCC..DDDDD:;
                      AAA..BBB,
                      CCC..DDD,
                      EEE..FFF:;
                      AAAAAAAAAAAA
                          ..BBBBBBBBBBBBBB,
                      CCCC,
                      DDDD:;
                      AAAAAAAAAA..BBBBBBBBBBB,
                      BBBBBBBBBBBBB
                          ..CCCCCCCCCCCCC,
                      EEEEEEEEEEE..FFFFFFFFFFF:;
                      AAAAAAAAAA..BBBBBBBBBBB,
                      CCCCCCCCCCCCC
                          ..DDDDDDDDDDDDDD:;
                    end;
                ",
                case_statement = "
                    case A of
                      AAAAAAAAAAAAAA: begin
                        Foo;
                      end;
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          Foo;
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          Fooooooooooo;
                      AAAAAAA, BBBBB: begin
                      end;
                      AAAAAAAAAAA, BBBBBBBBB:
                      begin
                      end;
                      AAAA..BBBB, CCC..DDD: begin
                      end;
                      AAAA..BBBB, CCCC..DDDD:
                      begin
                      end;
                      AAAAAAAAAAAAAA:
                        if AAAAA then begin
                          Foo
                        end
                        else
                          Bar;
                    else
                      Baz;
                    end;
                ",
                case_if_else = "
                    case A of
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          Foo;
                    else
                      Bar;
                    end;
                    case A of
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          Foo
                        else
                          Bar;
                    end;
                    case A of
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          while BBBBB do
                            Foo;
                    else
                      Bar;
                    end;
                    case A of
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          while BBBBB do
                            Foo
                        else
                          Bar;
                    end;
                    case A of
                      AAAAAAAAAAAAAA:
                        if AAAAA then
                          Foo
                        else
                          Bar;
                    else
                      Baz;
                    end;
                ",
            );
        }
    }

    mod except_on {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                on_name = "
                    try
                    except
                      on AAAAAAAA do
                        AAAAAAAAAAAAA;
                      on AAAAAAAAAAAAAAAA do begin
                        A;
                      end;
                      on AAAAAAAAAAAAAAAAA do
                      begin
                        A;
                      end;
                      on AAAAAAAAA.AAAAAAAAAAAA do
                        B;
                      on AAAAAAAAA
                          .AAAAAAAAAAAAA do
                        B;
                      on AAAAAAAA.AAAAAAA do begin
                        B;
                      end;
                      on AAAAAAAA.AAAAAAAA do
                      begin
                        B;
                      end;
                      on AAAAAAAAAAAA
                          .AAAAAAAAAA do
                      begin
                        B;
                      end;
                      on AAAAAA.AAAAAAAA.AAAAAA do
                        B;
                      on AAAAAA
                          .AAAAAAAA
                          .AAAAAAA do
                        B;
                      on AAAAA.AAAAA.AAAA do begin
                        B;
                      end;
                      on AAAAA.AAAAA.AAAAA do
                      begin
                        B;
                      end;
                      on AAAAAAA
                          .AAAAAAA
                          .AAAAAAA do
                      begin
                        B;
                      end;
                    end;
                ",
            );
        }
    }

    mod repeat_until {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                until = "
                    repeat
                    until AAAAAAAAAAAAAAAAAAAAAAA;
                    repeat
                    until AAAAAAAAAAAAAAAAAAAAAAAAAA;
                    repeat
                    until AAAAAAAAAAA.BBBBBBBBBBB;
                    repeat
                    until AAAAAAAAAAA
                        .BBBBBBBBBBBB;
                    repeat
                    until AAAAAAAAAAA.BBBBBB(111);
                    repeat
                    until AAAAAAAAAAA.BBBBBBB(
                        1111);
                    repeat
                    until AAAAAAAAAAAA
                        .BBBBBBBBBBBBB;
                    repeat
                    until AAAAAAAAAAA.BBBBBBBBB(
                        111);
                    repeat
                    until AAAAAAAAAAA
                        .BBBBBBBB(11111, 11111);
                    repeat
                    until AAAAAAAAAAA
                        .BBBBBBBBBBBB(
                            111111,
                            111111)
                        .CCC;
                ",
            );
        }
    }
}

mod statements {
    use super::*;

    pub fn generate(root_dir: &Path) {
        identifier_chaining::generate(root_dir);
        named_args::generate(root_dir);
        inline_declarations::generate(root_dir);
        goto::generate(root_dir);
        raise::generate(root_dir);
        assign::generate(root_dir);
    }

    mod identifier_chaining {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                no_args = "
                    applesssssssss.bananassssssss;
                    applesssssssss
                        .bananasssssssss;
                    applesssss.bananassss.carrots;
                    applesssss
                        .bananassss
                        .carrotss;
                ",
                empty_arg_list = "
                    applesssssssss.bananassssss();
                    applesssssssss
                        .bananasssssss();
                    applesssss.bananass.carrots();
                    applesssss
                        .bananass
                        .carrotss();
                ",
                end_arg_list = "
                    applesssssss.bananassss(a, b);
                    applesssssss
                        .bananasssss(a, b);
                    apples.bananassssssssssssssss(
                        a,
                        b
                    );
                    appless
                        .bananassssssssssssssss(
                            a,
                            b
                        );
                    appless.bananas.carrots(a, b);
                    apples.bananas.carrotss.dates(
                        a,
                        b
                    );
                    appless
                        .bananas
                        .carrotssssssss(a, b);
                    appless
                        .bananas
                        .carrotsssssssssssssss(
                            a,
                            b
                        );
                ",
                middle_arg_list = "
                    applessssssssss(a, b).carrots;
                    applesssssssssss(a, b)
                        .carrots;
                    applesssssssssss(
                            aaaaa,
                            bbbbbb)
                        .carrots;
                    appless.bananas(a, b).carrots;
                    appless
                        .bananas(a, b)
                        .carrotss;
                    appless
                        .bananas(
                            aaaaaaaaa,
                            bbbbbbbb)
                        .carrotss;
                ",
                many_arg_lists = "
                    apples.boo(a, b).carrot(c, d);
                    apples
                        .boo(a, b)
                        .carrots(c, d);
                    apples
                        .boo(
                            aaaaaaaaaaa,
                            bbbbbbbbbb)
                        .carrots(c, d);
                    apples
                        .boo(a, b)
                        .carrots(
                            cccccccc,
                            dddddddd);
                    apples
                        .boo(
                            aaaaaaaaaaa,
                            bbbbbbbbbb)
                        .carrots(
                            cccccccc,
                            dddddddd);
                ",
                nested = "
                    apples
                        .boo(
                            aaaaaaaaaaa,
                            carrotsss
                                .durianssss
                                .eggplantss(figs))
                        .grapes;
                ",
            );
        }
    }

    mod named_args {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                single = "
                    AAAAAAAAAAA(AAAAAA := BBBBBB);
                    AAAAAAAAAAA(
                        AAAAAAA := BBBBBBB
                    );
                    AAAAAAAAAAA(
                        AAAAAAA :=
                            BBBBBBBBBBBBBBBB
                    );
                ",
                multiple = "
                    AAAAAAAAAAAAA(A := B, C := D);
                    AAAAAAAAAAAAAA(
                        A := B,
                        C := D
                    );
                    AAAAAAAAAAAAAA(
                        AAAAAAAAAAAA :=
                            BBBBBBBBBB,
                        CCCCCCCCCCCC :=
                            DDDDDDDDDDD
                    );
                ",
                expression = "
                    AAAAAAAAAAA(AAAAAA := BB + B);
                    AAAAAAAAAAA(
                        AAAAAAA := BBBBBB + BBBBB
                    );
                    AAAAAAAAAAA(
                        AAAAAAA :=
                            BBBBBBBBB
                                + BBBBBBBBBBB
                    );
                ",
            );
        }
    }

    mod inline_declarations {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                untyped_const = "
                    begin
                      const AAAAAAAAAA = BBBBBBBB;
                      const AAAAAAAAAAA =
                          BBBBBBBB;
                      const AAAAAA = BBBBB + CCCC;
                      const AAAAAA =
                          BBBBB + CCCCC;
                      const AAAAAA =
                          BBBBBBBBBBB
                              + CCCCCCCCCC;
                      const AAAAAAAAAAAAAAAAAAAAAAA =
                          BBBBBBBBBBB
                              + CCCCCCCCCC;
                    end;
                ",
                typed_const = "
                    begin
                      const AAAAAA: BBBBB = CCCCC;
                      const AAAAAA: BBBBB =
                          CCCCCC;
                      const AAAAAA: BBBBB.BBBBB =
                          CCCCCC;
                      const AAAA: BBB = CCC + DDD;
                      const AAAA: BBB =
                          CCCC + DDD;
                      const AAAA: BBB =
                          CCCCCCCCCCCC
                              + DDDDDDDDDDD;
                      const AAAA:
                              BBBBBBBBBBBBBBB =
                          CCCC + DDD;
                      const AAAA:
                              BBBBBBB.BBBBBBB =
                          CCCC + DDD;
                      const AAAA:
                              BBBBBBBBBBB
                                  .BBBBBBBBBBB =
                          CCCC + DDD;
                      const AAAA:
                              BBBBBBBBBBBBBBB =
                          CCCCCCCCCCC
                              + DDDDDDDDDDD;
                      const AAAA:
                              BBBBBBBBBBB
                                  .BBBBBBBBBBB =
                          CCCCCCCCC
                              + DDDDDDDDDDDD;
                      const AAAAAAAAAAAAAAAAAAAAAAA:
                              BBBBBBBBBBB
                                  .BBBBBBBBBBB =
                          CCCCCCCCC
                              + DDDDDDDDDDDD;
                    end;
                ",
                untyped_var = "
                    begin
                      var AAAAAAAAAA := BBBBBBBB;
                      var AAAAAAAAAAA :=
                          BBBBBBBBBB;
                      var AAAAAA := BBBBB + CCCC;
                      var AAAAAA :=
                          BBBBBB + CCCCCC;
                      var AAAAAAAAAAAAAAAAAAAAAAAAA :=
                          BBBBBBBBBBB
                              + CCCCCCCCCC;
                      var AAAAAAAA :=
                          BBBBBBBBBBB
                              + CCCCCCCCCC;
                    end;
                ",
                typed_var = "
                    begin
                      var AAAAAA: BBBBB := CCCCC;
                      var AAAAAA: BBBBB :=
                          CCCCCCC;
                      var AAAA: BBB := CCC + DDD;
                      var AAAA: BBB :=
                          CCCC + DDDD;
                      var AAAA: BBB :=
                          CCCCCCCCCCCC
                              + DDDDDDDDDDD;
                      var AAAAA:
                              BBBBBBBBBBBBBBB :=
                          CCCC + DDD;
                      var AAAAA:
                              BBBBBBB.BBBBBBB :=
                          CCCCCCCCCCC
                              + DDDDDDDDDD;
                      var AAAAA:
                              BBBBBBBBBBB
                                  .BBBBBBBBBB :=
                          CCCCCCCCCCC
                              + DDDDDDDDDD;
                      var AAAAA:
                              BBBBBBBBBBBBBBB :=
                          CCCCCCCCCCC
                              + DDDDDDDDDD;
                      var AAAAAAAAAAAAAAAAAAAAAAAAA:
                              BBBBBBBBBBBBBBB :=
                          CCCCCCCCCCC
                              + DDDDDDDDDD;
                    end;
                ",
            );
        }
    }

    mod goto {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                goto = "
                    goto AAAAAAAAAAAAAAAAAAAAAAAA;
                    goto
                        AAAAAAAAAAAAAAAAAAAAAAAAA;
                    goto
                        AAAAAAAAAAAAAAAAAAAAAAAAAA;
                    goto 111111111111111111111111;
                    goto
                        1111111111111111111111111;
                    goto
                        11111111111111111111111111;
                ",
                label = "
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAA:
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA:
                    11111111111111111111111111111:
                    111111111111111111111111111111:
                ",
            );
        }
    }

    mod raise {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                raise = "
                    raise;
                    raise AAAAAAAAAAAAAAAAAAAAAAA;
                    raise AAAAAAAAAAAAAAAAAAAAAAAA;
                    raise AAAAAAAAAA(BBBB, CCCCC);
                    raise AAAAAAAAAAAA(
                        BBBBB,
                        CCCCC);
                    raise AAAAAA.AAAAAA(BBB, CCC);
                    raise AAAAAAAA
                        .AAAAAAA(BBB, CCC);
                    raise AAAAAAA.AAAAAAAAAAAAAAA(
                        BBB,
                        CCC);
                    raise AAAAAAAA
                        .AAAAAAAAAAAAAAAAA(
                            BBB,
                            CCC);
                ",
                raise_at = "
                    raise AAAAAAAAAA at AAAAAAAAA;
                    raise AAAAAAAAAA at AAA + AAA;
                    raise A(BB, CC) at AAAAAAAAAA;
                    raise AA(BB, CC) at AAA + AAA;
                    raise A.A(BBB, CCC) at DDDDDD;
                    raise AA.A(B, C) at DDD + DDD;

                    raise AAAAAAAAAAA
                        at AAAAAAAAA;
                    raise AAAAAAAAAAA
                        at AAA + AAA;
                    raise AAAAAAAAAAA
                        at AAAAAAAAAA
                            + AAAAAAAAAA;
                    raise AAAAAAAAAAA
                        at AAAAAAAAAA(
                            BBBBB,
                            CCCCC);
                    raise AA(BB, CC)
                        at AAAAAAAAAA;
                    raise AAAAAAAAAAA(
                            BBBBBB,
                            CCCCCC)
                        at AAAAAAAAAA;
                    raise AAA(BB, CC)
                        at AAA + AAA;
                    raise AA.A(BBB, CCC)
                        at DDDDDD;
                    raise AAA.A(B, C)
                        at DDD + DDD;
                ",
            );
        }
    }

    mod assign {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                break_after_eq = "
                    AAAAAAAAAAA := BBBBBBBBBBBBBB;
                    AAAAAAAAAAA :=
                        BBBBBBBBBBBBBBB;
                    AAAAAAAAAAA :=
                        BBBBB
                            + BBBBB
                            + BBBBB
                            + BBBBB;
                ",
                break_before_eq = "
                    (AAAAAAAAAAAAAAAA
                            + BBBBBBB) :=
                        CCCCC;
                    (AAAA + BBBB + CCCC).DD := EE;
                    (AAAA + BBBB + CCCC).DDD :=
                        EEEEEEEEEEE + EEEEEEEEEEE;
                    (AAAA + BBBB + CCCC).DDD :=
                        EEEEEEEEEEE
                            + EEEEEEEEEEEE;
                    (AAAAA + BBBBB + CCCCC)
                            .DDDD :=
                        EEEEEEEEEEE + EEEEEEEEEEE;
                    (AAAAAAAA
                                + BBBBBBBB
                                + CCCCCCC)
                            .DDDD :=
                        EEEEEEEEEEE + EEEEEEEEEEE;
                    (AAAAAAAA
                                + BBBBBBBB
                                + CCCCCCC)
                            .DDDD :=
                        EEEEEEEEEEE
                            + EEEEEEEEEEEE;
                ",
            );
        }
    }
}

mod expressions {
    use super::*;

    pub fn generate(root_dir: &Path) {
        boolean::generate(root_dir);
        mathematical::generate(root_dir);
        mixed::generate(root_dir);
        unary::generate(root_dir);
        set::generate(root_dir);
    }

    mod boolean {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                and = "
                    applesssssssss and bananassss;
                    applesssssssss
                        and bananasssss;
                ",
                ands = "
                    apples and bananas and carrot;
                    apples
                        and bananas
                        and carrots;
                ",
                or = "
                    applesssssssss or bananasssss;
                    applesssssssss
                        or bananassssss;
                ",
                ors = "
                    appless or bananas or carrots;
                    appless
                        or bananas
                        or carrotss;
                ",
                and_or = "
                    a and b or c;
                    appless and bananas
                        or carrots;
                    applesssssssss
                            and bananassssss
                        or carrots;
                ",
                ands_or = "
                    a and b and c or d;
                    apples and bananas and carrots
                        or durian;
                    appless
                            and bananas
                            and carrots
                        or durian;
                ",
                ors_and = "
                    aaaaaaa or bbbbbbb or c and d;
                    apple
                        or banana
                        or carrot and durian;
                    apple
                        or banana
                        or carrotssss
                            and durianssss;
                ",
                mixed = "
                    aaaa and bbbb or cccc and ddd;
                    apple and banana
                        or carrot and durian;
                    applessssssss
                            and bananasssssss
                        or carrotsssss
                            and duriansss;
                ",
                with_parens = "
                    AAAAAAA :=
                        AAAA
                            and ((BBBBBBBBBBBBBB
                                and CCCCCCCC))
                            and DDDDDDD;

                ",
            );
        }
    }

    mod mathematical {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                mul = "
                    applessssssssss * bananasssss;
                    applessssssssss
                        * bananassssss;
                ",
                div = "
                    applessssssssss / bananasssss;
                    applessssssssss
                        / bananassssss;
                ",
                muls = "
                    appless * bananass * carrotss;
                    applesss
                        * bananass
                        * carrotss;
                ",
                divs = "
                    appless / bananass / carrotss;
                    applesss
                        / bananass
                        / carrotss;
                ",
                plus = "
                    applessssssssss + bananasssss;
                    applessssssssss
                        + bananassssss;
                ",
                minus = "
                    applessssssssss - bananasssss;
                    applessssssssss
                        - bananassssss;
                ",
                pluses = "
                    appless + bananass + carrotss;
                    applesss
                        + bananass
                        + carrotss;
                ",
                minuses = "
                    appless - bananass - carrotss;
                    applesss
                        - bananass
                        - carrotss;
                ",
                mul_plus = "
                    appless * bananass + carrotss;
                    appless * bananasss
                        + carrotss;
                    applessssssssss
                            * bananasssssss
                        + carrots;
                ",
                muls_plus = "
                    apple * boo * carrot + durian;
                ",
                plus_mul = "
                    appless + bananass * carrotss;
                    appless
                        + bananass * carrotsss;
                    appless
                        + bananasssss
                            * carrotssssss;
                ",
                pluses_mul = "
                    a + b + c * d;
                    apple + boo + carrot * durian;
                    apple
                        + banana
                        + carrotsssss * duriassss;
                    apple
                        + banana
                        + carrotssssss
                            * duriansssss;
                ",
                mixed = "
                    a * b + c * d;
                    apple * boo + carrot * durian;
                    apple * banana
                        + carrot * durian;
                    applesssssssss
                            * bananassssssss
                        + carrotsssssss
                            * durianssss;
                ",
            );
        }
    }

    mod mixed {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                routine_maths = "
                    applesssssssssss(aaaaa, bbbbb)
                        + cccc
                        + dddd;
                    applesss.bananas(aaaaa, bbbbb)
                        + cccc
                        + dddd;
                    applesssssssssss(
                            aaaaaa,
                            bbbbbb)
                        + cccc
                        + dddd;
                    applesssssssssss(
                            (aaaaaa),
                            (bbbbbb))
                        > -1;
                    applesssss
                            .bananas(aaaaa, bbbbb)
                        + cccc
                        + dddd;
                    applesssssssssss
                            .bananasssssss(
                                aaaaaa,
                                bbbbbb)
                        + cccc
                        + dddd;
                    ABC :=
                        applesssssssssss(aaa, bbb)
                            + cccc
                            + dddd;
                    ABC :=
                        applesss.bananas(aaa, bbb)
                            + cccc
                            + dddd;
                    ABC :=
                        applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            + cccc
                            + dddd;
                    ABC :=
                        applesssss
                                .bananas(aaa, bbb)
                            + cccc
                            + dddd;
                    ABC :=
                        applesssssssssss
                                .bananasssssss(
                                    aaaaaa,
                                    bbbbbb)
                            + cccc
                            + dddd;
                    ABC :=
                        applesssssss.bananasss(
                            CCCCCCCC()
                                and DDDDDDD()
                        );
                ",
                parens_maths = "
                    (aaaaa + bbbbb) + cccc + dddd;
                    (aaaaaa + bbbbb)
                        + cccc
                        + dddd;
                    (aaaaaa
                            + bbbbb
                            + aaaaa
                            + bbbb)
                        + cccc
                        + dddd;
                    ABC :=
                        (aaaaaa + bbbbb)
                            + cccc
                            + dddd;
                    ABC :=
                        (aaaaaa
                                + bbbbb
                                + aaaaa
                                + bbbb)
                            + cccc
                            + dddd;
                    ABC :=
                        aaaaaaaaaaaa
                            + (aaaaaaaaaaaaaaaaaaa
                                + bbbb);
                    ABC :=
                        aaaaaaaaaaaa
                            + (aaaaaaaaaaaaaaaaaaaa
                                + bbbbb);
                ",
                deref = "
                    aaaaaa := aaaaaaaaaaaaaaaaaa^;
                    aaaaaaa :=
                        aaaaaaaaaaaaaaaaaa^;
                    aaaaaaa :=
                        aaaaaaaaaaaaaaaaaaaaaaaaa^;
                ",
            );
        }
    }

    mod unary {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                boolean = "
                    AAAAAAAAA := not AAAAAAAAAA;
                    AAAAAAAAA :=
                        not AAAAAAAAAAAAA;
                    AAAAAAAAA :=
                        not {} AAAAAAAAAAAAAAAAAA;
                    AAAAAAAAA :=
                        not {} AAAAAAAAAAAAAAAAAAA;
                    AAAAAAAAA :=
                        not AAAAAAA and not BBBBB;
                    AAAAAAAAA :=
                        not AAAAAAA
                            and not BBBBBB;
                    AAAAAAAAA :=
                        not AAAAAAA
                            and not BBBBBBBBBBBBBB;
                    not applesssssssssss(
                        aaaaaa,
                        bbbbbb
                    );
                    not applesssssssssss(
                            aaaaaa,
                            bbbbbb)
                        .ccccccccccc(
                            dddddd,
                            eeeeee);
                    ABC :=
                        not applesssssssssss(
                            aaaaaa,
                            bbbbbb
                        );
                    ABC :=
                        not applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            .ccccccccccc(
                                dddddd,
                                eeeeee);
                ",
                maths = "
                    +applesssssssssss(
                        aaaaaa,
                        bbbbbb
                    );
                    -applesssssssssss(
                        aaaaaa,
                        bbbbbb
                    );
                    -applesssssssssss(
                            aaaaaa,
                            bbbbbb)
                        .ccccccccccc(
                            dddddd,
                            eeeeee);
                    ABC :=
                        -applesssssssssss(
                            aaaaaa,
                            bbbbbb
                        );
                    ABC :=
                        -applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            .ccccccccccc(
                                dddddd,
                                eeeeee);
                    ABC :=
                        -applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            + carrotttsssss;
                    ABC :=
                        -applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            + -carrotttsssss;
                ",
                address = "
                    @applesssssssssss(
                        aaaaaa,
                        bbbbbb
                    );
                    @applesssssssssss(
                            aaaaaa,
                            bbbbbb)
                        .ccccccccccc(
                            dddddd,
                            eeeeee);
                    ABC :=
                        @applesssssssssss(
                            aaaaaa,
                            bbbbbb
                        );
                    ABC :=
                        @applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            .ccccccccccc(
                                dddddd,
                                eeeeee);
                    ABC :=
                        @applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            + carrotttsssss;
                    ABC :=
                        @applesssssssssss(
                                aaaaaa,
                                bbbbbb)
                            + @carrotttsssss;
                    ABC := @ {} AAAAAAAAAAAAAAAAA;
                    ABC :=
                        @ {} AAAAAAAAAAAAAAAAAA;
                    ABC :=
                        @ {} AAAAAAAAAAAAAAAAAAAAA;
                ",
            );
        }
    }

    mod set {
        use super::*;

        pub fn generate(root_dir: &Path) {
            generate_test_cases!(
                root_dir,
                assignment = "
                    AAAA := [EEEEE, EEEEE, EEEEE];
                    AAAAA :=
                        [EEEEE, EEEEE, EEEEE];
                    AAAAA :=
                        [
                            EEEEEEE,
                            EEEEEEE,
                            EEEEEE
                        ];
                    AA := [EE, EE, EE] + [EE, EE];
                    AAA :=
                        [EE, EE, EE] + [EE, EE];
                    AAA :=
                        [EEE, EEE, EEE]
                            + [EEE, EEE];
                    AAA :=
                        [
                                EEEEEEE,
                                EEEEEEE,
                                EEEEEEE]
                            + [EEE, EEE];
                    AAA :=
                        [EEE, EEE, EEE]
                            + [
                                EEEEEEEE,
                                EEEEEEEE];
                    AAA :=
                        [
                                EEEEEEE,
                                EEEEEEE,
                                EEEEEEE]
                            + [
                                EEEEEEEE,
                                EEEEEEEE];
                ",
                argument = "
                    AAAAAA([EEEEE, EEEEE, EEEEE]);
                    AAAAAAA(
                        [EEEEE, EEEEE, EEEEE]
                    );
                    AAAAAAA(
                        [
                            EEEEEEE,
                            EEEEEEE,
                            EEEEEEE
                        ]
                    );
                    AAAA([EE, EE, EE] + [EE, EE]);
                    AAAAA(
                        [EE, EE, EE] + [EE, EE]
                    );
                    AAAAA(
                        [EEE, EEE, EEE]
                            + [EEE, EEE]
                    );
                    AAAAA(
                        [
                                EEEEEEE,
                                EEEEEEE,
                                EEEEEEE]
                            + [EEE, EEE]
                    );
                    AAAAA(
                        [EEE, EEE, EEE]
                            + [
                                EEEEEEEEE,
                                EEEEEEEEE]
                    );
                    AAAAA(
                        [
                                EEEEEEE,
                                EEEEEEE,
                                EEEEEEE]
                            + [
                                EEEEEEEEE,
                                EEEEEEEEE]
                    );
                ",
            );
        }
    }
}

mod attributes {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            ident = "
                    [AAAAAAAAAAAAAAAAAAAAAAAAAAAA]
                    [
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    ]
                    [AAAAAAAA, AAAAAAA, AAAAAAAAA]
                    [
                        AAAAAAAA,
                        AAAAAAAA,
                        AAAAAAAAAA
                    ]
            ",
            constructor = "
                    [AAAAAAAAAAAAAA(AAAAA, AAAAA)]
                    [
                        AAAAAAAAAAAAAAA(
                            AAAAA,
                            AAAAA
                        )
                    ]
                    [
                        AAAAAAAAAAAAAAA(
                            AAAAA,
                            AAAAA
                        ),
                        AAAAAAAAAA(AAA, AAA, AAA),
                        AAAAAAAAAA(AAA, AAA, AAA)
                    ]
                    [
                        AAAAAAAAAA(AAA, AAA, AAA),
                        AAAAAAAAAAAAAAA(
                            AAAAA,
                            AAAAA
                        ),
                        AAAAAAAAAA(AAA, AAA, AAA)
                    ]
                    [
                        AAAAAAAAAA(AAA, AAA, AAA),
                        AAAAAAAAAA(AAA, AAA, AAA),
                        AAAAAAAAAAAAAAAA(
                            AAAAA,
                            AAAAA
                        )
                    ]
            ",
        );
    }
}

mod line_length_violations {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            assignment = "
                    EEEEEEEEE + EEEEEEEEEEEEEEE :=
                        E;

                    EEEEEEEEE
                            + EEEEEEEEEEEEEEEE :=
                        E;

                    EEEEEEEEE + EEEEEEEEEEEEEEEEEE :=
                        E;

                    EEEEEEEEE
                            + EEEEEEEEEEEEEEEEEEE :=
                        E;

                    EEEEEEEEE + EEEEEEEEEEEEEEEEEEEEE :=
                        E;
            ",
            trailing_comments = "
                    AAAAAAAAA :=
                        AAAAAAAAAA // -------------
                            + BBBBBBBBBB // -------
                            + CCCCCCCCCC // -------
                            + DDDDDDDDDD; // ------
            ",
        );
    }
}

mod regression {
    use super::*;

    pub fn generate(root_dir: &Path) {
        generate_test_cases!(
            root_dir,
            space_between_keywords_with_newline_removal = "
                // In this form, the pass that is
                // the routine declaration wins.
                {$ifdef A}
                foo(
                {$else}
                {$endif}
                procedure a;
                begin
                end
                {$ifdef A}
                );
                {$else}
                {$endif}

                // In this form, the pass that is
                // the (invalid) anonymous routine
                // declaration wins.
                {$ifdef A}
                {$else}
                foo(
                {$endif}
                    procedure a;
                    begin end
                {$ifdef A}
                {$else}
                );
                {$endif}
            ",
        );
    }
}
