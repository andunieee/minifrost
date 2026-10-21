// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'engine.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$Finished {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Finished);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'Finished()';
}


}

/// @nodoc
class $FinishedCopyWith<$Res>  {
$FinishedCopyWith(Finished _, $Res Function(Finished) __);
}


/// Adds pattern-matching-related methods to [Finished].
extension FinishedPatterns on Finished {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Finished_KeyCreated value)?  keyCreated,TResult Function( Finished_Signed value)?  signed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that);case Finished_Signed() when signed != null:
return signed(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Finished_KeyCreated value)  keyCreated,required TResult Function( Finished_Signed value)  signed,}){
final _that = this;
switch (_that) {
case Finished_KeyCreated():
return keyCreated(_that);case Finished_Signed():
return signed(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Finished_KeyCreated value)?  keyCreated,TResult? Function( Finished_Signed value)?  signed,}){
final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that);case Finished_Signed() when signed != null:
return signed(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String keyId,  String name)?  keyCreated,TResult Function( String txid,  String txHex)?  signed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that.keyId,_that.name);case Finished_Signed() when signed != null:
return signed(_that.txid,_that.txHex);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String keyId,  String name)  keyCreated,required TResult Function( String txid,  String txHex)  signed,}) {final _that = this;
switch (_that) {
case Finished_KeyCreated():
return keyCreated(_that.keyId,_that.name);case Finished_Signed():
return signed(_that.txid,_that.txHex);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String keyId,  String name)?  keyCreated,TResult? Function( String txid,  String txHex)?  signed,}) {final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that.keyId,_that.name);case Finished_Signed() when signed != null:
return signed(_that.txid,_that.txHex);case _:
  return null;

}
}

}

/// @nodoc


class Finished_KeyCreated extends Finished {
  const Finished_KeyCreated({required this.keyId, required this.name}): super._();
  

 final  String keyId;
 final  String name;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Finished_KeyCreatedCopyWith<Finished_KeyCreated> get copyWith => _$Finished_KeyCreatedCopyWithImpl<Finished_KeyCreated>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Finished_KeyCreated&&(identical(other.keyId, keyId) || other.keyId == keyId)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,keyId,name);
}

@override
String toString() {
    return 'Finished.keyCreated(keyId: $keyId, name: $name)';
}


}

/// @nodoc
abstract mixin class $Finished_KeyCreatedCopyWith<$Res> implements $FinishedCopyWith<$Res> {
  factory $Finished_KeyCreatedCopyWith(Finished_KeyCreated value, $Res Function(Finished_KeyCreated) _then) = _$Finished_KeyCreatedCopyWithImpl;
@useResult
$Res call({
 String keyId, String name
});




}
/// @nodoc
class _$Finished_KeyCreatedCopyWithImpl<$Res>
    implements $Finished_KeyCreatedCopyWith<$Res> {
  _$Finished_KeyCreatedCopyWithImpl(this._self, this._then);

  final Finished_KeyCreated _self;
  final $Res Function(Finished_KeyCreated) _then;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? keyId = null,Object? name = null,}) {
  return _then(Finished_KeyCreated(
keyId: null == keyId ? _self.keyId : keyId // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Finished_Signed extends Finished {
  const Finished_Signed({required this.txid, required this.txHex}): super._();
  

 final  String txid;
 final  String txHex;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Finished_SignedCopyWith<Finished_Signed> get copyWith => _$Finished_SignedCopyWithImpl<Finished_Signed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Finished_Signed&&(identical(other.txid, txid) || other.txid == txid)&&(identical(other.txHex, txHex) || other.txHex == txHex));
}


@override
int get hashCode {
    return Object.hash(runtimeType,txid,txHex);
}

@override
String toString() {
    return 'Finished.signed(txid: $txid, txHex: $txHex)';
}


}

/// @nodoc
abstract mixin class $Finished_SignedCopyWith<$Res> implements $FinishedCopyWith<$Res> {
  factory $Finished_SignedCopyWith(Finished_Signed value, $Res Function(Finished_Signed) _then) = _$Finished_SignedCopyWithImpl;
@useResult
$Res call({
 String txid, String txHex
});




}
/// @nodoc
class _$Finished_SignedCopyWithImpl<$Res>
    implements $Finished_SignedCopyWith<$Res> {
  _$Finished_SignedCopyWithImpl(this._self, this._then);

  final Finished_Signed _self;
  final $Res Function(Finished_Signed) _then;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? txid = null,Object? txHex = null,}) {
  return _then(Finished_Signed(
txid: null == txid ? _self.txid : txid // ignore: cast_nullable_to_non_nullable
as String,txHex: null == txHex ? _self.txHex : txHex // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
