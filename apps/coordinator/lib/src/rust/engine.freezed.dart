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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Finished_KeyCreated value)?  keyCreated,TResult Function( Finished_Signed value)?  signed,TResult Function( Finished_BackupRecorded value)?  backupRecorded,TResult Function( Finished_BackupChecked value)?  backupChecked,TResult Function( Finished_WalletRestored value)?  walletRestored,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that);case Finished_Signed() when signed != null:
return signed(_that);case Finished_BackupRecorded() when backupRecorded != null:
return backupRecorded(_that);case Finished_BackupChecked() when backupChecked != null:
return backupChecked(_that);case Finished_WalletRestored() when walletRestored != null:
return walletRestored(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Finished_KeyCreated value)  keyCreated,required TResult Function( Finished_Signed value)  signed,required TResult Function( Finished_BackupRecorded value)  backupRecorded,required TResult Function( Finished_BackupChecked value)  backupChecked,required TResult Function( Finished_WalletRestored value)  walletRestored,}){
final _that = this;
switch (_that) {
case Finished_KeyCreated():
return keyCreated(_that);case Finished_Signed():
return signed(_that);case Finished_BackupRecorded():
return backupRecorded(_that);case Finished_BackupChecked():
return backupChecked(_that);case Finished_WalletRestored():
return walletRestored(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Finished_KeyCreated value)?  keyCreated,TResult? Function( Finished_Signed value)?  signed,TResult? Function( Finished_BackupRecorded value)?  backupRecorded,TResult? Function( Finished_BackupChecked value)?  backupChecked,TResult? Function( Finished_WalletRestored value)?  walletRestored,}){
final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that);case Finished_Signed() when signed != null:
return signed(_that);case Finished_BackupRecorded() when backupRecorded != null:
return backupRecorded(_that);case Finished_BackupChecked() when backupChecked != null:
return backupChecked(_that);case Finished_WalletRestored() when walletRestored != null:
return walletRestored(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String keyId,  String name)?  keyCreated,TResult Function( String txid,  String txHex)?  signed,TResult Function( String keyName,  int shareIndex)?  backupRecorded,TResult Function( String keyName,  int shareIndex)?  backupChecked,TResult Function( String keyId,  String name)?  walletRestored,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that.keyId,_that.name);case Finished_Signed() when signed != null:
return signed(_that.txid,_that.txHex);case Finished_BackupRecorded() when backupRecorded != null:
return backupRecorded(_that.keyName,_that.shareIndex);case Finished_BackupChecked() when backupChecked != null:
return backupChecked(_that.keyName,_that.shareIndex);case Finished_WalletRestored() when walletRestored != null:
return walletRestored(_that.keyId,_that.name);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String keyId,  String name)  keyCreated,required TResult Function( String txid,  String txHex)  signed,required TResult Function( String keyName,  int shareIndex)  backupRecorded,required TResult Function( String keyName,  int shareIndex)  backupChecked,required TResult Function( String keyId,  String name)  walletRestored,}) {final _that = this;
switch (_that) {
case Finished_KeyCreated():
return keyCreated(_that.keyId,_that.name);case Finished_Signed():
return signed(_that.txid,_that.txHex);case Finished_BackupRecorded():
return backupRecorded(_that.keyName,_that.shareIndex);case Finished_BackupChecked():
return backupChecked(_that.keyName,_that.shareIndex);case Finished_WalletRestored():
return walletRestored(_that.keyId,_that.name);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String keyId,  String name)?  keyCreated,TResult? Function( String txid,  String txHex)?  signed,TResult? Function( String keyName,  int shareIndex)?  backupRecorded,TResult? Function( String keyName,  int shareIndex)?  backupChecked,TResult? Function( String keyId,  String name)?  walletRestored,}) {final _that = this;
switch (_that) {
case Finished_KeyCreated() when keyCreated != null:
return keyCreated(_that.keyId,_that.name);case Finished_Signed() when signed != null:
return signed(_that.txid,_that.txHex);case Finished_BackupRecorded() when backupRecorded != null:
return backupRecorded(_that.keyName,_that.shareIndex);case Finished_BackupChecked() when backupChecked != null:
return backupChecked(_that.keyName,_that.shareIndex);case Finished_WalletRestored() when walletRestored != null:
return walletRestored(_that.keyId,_that.name);case _:
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

/// @nodoc


class Finished_BackupRecorded extends Finished {
  const Finished_BackupRecorded({required this.keyName, required this.shareIndex}): super._();
  

 final  String keyName;
 final  int shareIndex;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Finished_BackupRecordedCopyWith<Finished_BackupRecorded> get copyWith => _$Finished_BackupRecordedCopyWithImpl<Finished_BackupRecorded>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Finished_BackupRecorded&&(identical(other.keyName, keyName) || other.keyName == keyName)&&(identical(other.shareIndex, shareIndex) || other.shareIndex == shareIndex));
}


@override
int get hashCode {
    return Object.hash(runtimeType,keyName,shareIndex);
}

@override
String toString() {
    return 'Finished.backupRecorded(keyName: $keyName, shareIndex: $shareIndex)';
}


}

/// @nodoc
abstract mixin class $Finished_BackupRecordedCopyWith<$Res> implements $FinishedCopyWith<$Res> {
  factory $Finished_BackupRecordedCopyWith(Finished_BackupRecorded value, $Res Function(Finished_BackupRecorded) _then) = _$Finished_BackupRecordedCopyWithImpl;
@useResult
$Res call({
 String keyName, int shareIndex
});




}
/// @nodoc
class _$Finished_BackupRecordedCopyWithImpl<$Res>
    implements $Finished_BackupRecordedCopyWith<$Res> {
  _$Finished_BackupRecordedCopyWithImpl(this._self, this._then);

  final Finished_BackupRecorded _self;
  final $Res Function(Finished_BackupRecorded) _then;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? keyName = null,Object? shareIndex = null,}) {
  return _then(Finished_BackupRecorded(
keyName: null == keyName ? _self.keyName : keyName // ignore: cast_nullable_to_non_nullable
as String,shareIndex: null == shareIndex ? _self.shareIndex : shareIndex // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Finished_BackupChecked extends Finished {
  const Finished_BackupChecked({required this.keyName, required this.shareIndex}): super._();
  

 final  String keyName;
 final  int shareIndex;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Finished_BackupCheckedCopyWith<Finished_BackupChecked> get copyWith => _$Finished_BackupCheckedCopyWithImpl<Finished_BackupChecked>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Finished_BackupChecked&&(identical(other.keyName, keyName) || other.keyName == keyName)&&(identical(other.shareIndex, shareIndex) || other.shareIndex == shareIndex));
}


@override
int get hashCode {
    return Object.hash(runtimeType,keyName,shareIndex);
}

@override
String toString() {
    return 'Finished.backupChecked(keyName: $keyName, shareIndex: $shareIndex)';
}


}

/// @nodoc
abstract mixin class $Finished_BackupCheckedCopyWith<$Res> implements $FinishedCopyWith<$Res> {
  factory $Finished_BackupCheckedCopyWith(Finished_BackupChecked value, $Res Function(Finished_BackupChecked) _then) = _$Finished_BackupCheckedCopyWithImpl;
@useResult
$Res call({
 String keyName, int shareIndex
});




}
/// @nodoc
class _$Finished_BackupCheckedCopyWithImpl<$Res>
    implements $Finished_BackupCheckedCopyWith<$Res> {
  _$Finished_BackupCheckedCopyWithImpl(this._self, this._then);

  final Finished_BackupChecked _self;
  final $Res Function(Finished_BackupChecked) _then;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? keyName = null,Object? shareIndex = null,}) {
  return _then(Finished_BackupChecked(
keyName: null == keyName ? _self.keyName : keyName // ignore: cast_nullable_to_non_nullable
as String,shareIndex: null == shareIndex ? _self.shareIndex : shareIndex // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Finished_WalletRestored extends Finished {
  const Finished_WalletRestored({required this.keyId, required this.name}): super._();
  

 final  String keyId;
 final  String name;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Finished_WalletRestoredCopyWith<Finished_WalletRestored> get copyWith => _$Finished_WalletRestoredCopyWithImpl<Finished_WalletRestored>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Finished_WalletRestored&&(identical(other.keyId, keyId) || other.keyId == keyId)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,keyId,name);
}

@override
String toString() {
    return 'Finished.walletRestored(keyId: $keyId, name: $name)';
}


}

/// @nodoc
abstract mixin class $Finished_WalletRestoredCopyWith<$Res> implements $FinishedCopyWith<$Res> {
  factory $Finished_WalletRestoredCopyWith(Finished_WalletRestored value, $Res Function(Finished_WalletRestored) _then) = _$Finished_WalletRestoredCopyWithImpl;
@useResult
$Res call({
 String keyId, String name
});




}
/// @nodoc
class _$Finished_WalletRestoredCopyWithImpl<$Res>
    implements $Finished_WalletRestoredCopyWith<$Res> {
  _$Finished_WalletRestoredCopyWithImpl(this._self, this._then);

  final Finished_WalletRestored _self;
  final $Res Function(Finished_WalletRestored) _then;

/// Create a copy of Finished
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? keyId = null,Object? name = null,}) {
  return _then(Finished_WalletRestored(
keyId: null == keyId ? _self.keyId : keyId // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
