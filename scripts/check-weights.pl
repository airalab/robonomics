#!/usr/bin/env perl
# Weight policy enforcement (issue #645).
#
# Scans every FRAME pallet under `frame/*/src/lib.rs` and fails if it finds
# an extrinsic whose `#[pallet::weight(...)]` attribute does not route
# through a benchmarked `WeightInfo` implementation (i.e. `T::WeightInfo::*`
# or `<T as Config>::WeightInfo::*`), or a pallet that defines dispatchables
# but has no `WeightInfo` trait at all.
#
# This intentionally does not require every pallet to *contain* benchmarks
# inline, only that extrinsics are charged via `WeightInfo` rather than a
# hardcoded/placeholder weight such as `0`, `Weight::default()`, or
# `Weight::zero()`.

use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);

my $script_dir = dirname(abs_path($0));
my $root = abs_path("$script_dir/..");
my $frame_dir = "$root/frame";

my @violations;
my @pallets = sort glob("$frame_dir/*/src/lib.rs");

for my $lib_rs (@pallets) {
    check_pallet($lib_rs);
}

if (@violations) {
    print "Weight policy violations found:\n\n";
    print "  - $_\n" for @violations;
    print "\nEvery extrinsic must be weighed via a benchmarked `T::WeightInfo::*` function.\n";
    exit 1;
}

print "Weight policy OK: checked " . scalar(@pallets) . " pallets, no placeholder weights found.\n";
exit 0;

sub slurp {
    my ($path) = @_;
    open(my $fh, '<', $path) or die "cannot open $path: $!";
    local $/;
    my $content = <$fh>;
    close $fh;
    return $content;
}

sub check_pallet {
    my ($lib_rs) = @_;
    my $text = slurp($lib_rs);

    my $has_dispatchables = $text =~ /#\[pallet::call\]\s*(?:#\[[^\]]*\]\s*)*impl(?:<[^>]*>)?\s+[^\{]*\{\s*[^\}]*\bpub\s+fn\b/s;
    my $has_call_block = $has_dispatchables;
    my $attr = '#[pallet::weight(';
    my @attr_positions;
    my $pos = 0;
    while (1) {
        my $idx = index($text, $attr, $pos);
        last if $idx < 0;
        push @attr_positions, $idx;
        $pos = $idx + length($attr);
    }

    if ($has_call_block && !@attr_positions) {
        push @violations, "$lib_rs: pallet declares #[pallet::call] but has no #[pallet::weight(...)] attributes";
    }

    for my $start (@attr_positions) {
        my $open_paren = $start + length($attr) - 1;
        my $close_paren = find_matching_paren($text, $open_paren);
        my $expr = substr($text, $open_paren + 1, $close_paren - $open_paren - 1);
        my $lineno = line_of($text, $start);

        if ($expr !~ /WeightInfo::/) {
            $expr =~ s/^\s+|\s+$//g;
            if (my $reason = allow_reason($text, $start)) {
                print "  (allowed) $lib_rs:$lineno: `$expr` — $reason\n";
                next;
            }
            push @violations, "$lib_rs:$lineno: placeholder weight `$expr` does not use a benchmarked WeightInfo function";
        }
    }

    if ($has_call_block) {
        my $weights_rs = dirname($lib_rs) . "/weights.rs";
        if (!-e $weights_rs || slurp($weights_rs) !~ /trait WeightInfo/) {
            my $dir = dirname($lib_rs);
            push @violations, "$dir: pallet has dispatchables but no `WeightInfo` trait in weights.rs";
        }
    }
}

sub allow_reason {
    my ($text, $attr_start) = @_;
    my $before = substr($text, 0, $attr_start);
    my @lines = split /\n/, $before;
    # Walk upwards over blank lines / other attributes to find an immediately
    # preceding `// weight-policy-allow: <reason>` suppression comment.
    for (my $i = $#lines; $i >= 0 && $i >= $#lines - 5; $i--) {
        my $line = $lines[$i];
        next if $line =~ /^\s*$/;
        if ($line =~ /^\s*\/\/\s*weight-policy-allow:\s*(.+?)\s*$/) {
            return $1;
        }
        last unless $line =~ /^\s*(\/\/|#\[)/;
    }
    return undef;
}

sub find_matching_paren {
    my ($text, $open_idx) = @_;
    my $depth = 0;
    for (my $i = $open_idx; $i < length($text); $i++) {
        my $c = substr($text, $i, 1);
        if ($c eq '(') {
            $depth++;
        } elsif ($c eq ')') {
            $depth--;
            return $i if $depth == 0;
        }
    }
    die "unbalanced parentheses starting at $open_idx in";
}

sub line_of {
    my ($text, $idx) = @_;
    my $prefix = substr($text, 0, $idx);
    my $count = () = $prefix =~ /\n/g;
    return $count + 1;
}
