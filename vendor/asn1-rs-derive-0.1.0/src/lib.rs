mod sequence;
use sequence::*;

synstructure::decl_derive!([BerSequence, attributes(debug_derive)] => derive_ber_sequence);
synstructure::decl_derive!([DerSequence, attributes(debug_derive)] => derive_der_sequence);
